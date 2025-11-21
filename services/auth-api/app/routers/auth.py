from datetime import timedelta
import base64
import hashlib
import json
import os
import time

import pyotp
from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select, text
from sqlalchemy.orm import Session

from app.config import settings
from app.dependencies import get_db, get_redis
from app.models import Policy, User
from app.schemas import (
	DeviceLoginAuthorizeRequest,
	DeviceLoginPollRequest,
	DeviceLoginStartRequest,
	DeviceLoginStartResponse,
	LoginRequest,
	RefreshRequest,
	TokenResponse,
)
from app.security import decode_jwt, key_manager, verify_password

router = APIRouter(prefix="/auth", tags=["auth"])


def _fetch_user_roles(user_id: int, db: Session) -> list[str]:
	"""Fetch user roles from operators table in admin-api database"""
	try:
		# Query operators table directly (shared database)
		# Note: We need to use raw SQL since Operator model is in admin-api, not auth-api
		from sqlalchemy import text
		result = db.execute(
			text("SELECT role FROM operators WHERE user_id = :user_id"),
			{"user_id": user_id}
		)
		row = result.first()
		if row:
			role = row[0]
			return [role] if role else []
	except Exception as e:
		# Non-fatal - table might not exist yet or user might not be an operator
		print(f"Warning: Could not fetch roles for user {user_id}: {e}")
		pass
	return []


@router.get("/.well-known/jwks.json", include_in_schema=False)
def jwks() -> dict:
	return key_manager.get_all_public_jwks()


@router.post("/login")
def login(payload: LoginRequest, db: Session = Depends(get_db), redis_client = Depends(get_redis)) -> TokenResponse:
	user = db.execute(select(User).where(User.email == str(payload.email), User.is_deleted == False)).scalar_one_or_none()  # noqa: E712
	if user is None or not verify_password(payload.password, user.password_hash):
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid credentials")

	if user.totp_secret:
		if not payload.totp_code:
			raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="totp required")
		totp = pyotp.TOTP(user.totp_secret)
		if not totp.verify(payload.totp_code, valid_window=1):
			raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid totp")

	policy = db.execute(select(Policy).where(Policy.user_id == user.id)).scalar_one_or_none()
	max_conn = policy.max_active_connections if policy else 5

	# Fetch roles from operators table
	roles = _fetch_user_roles(user.id, db)
	
	claims = {"policy": {"max_active_connections": max_conn}}
	if roles:
		claims["roles"] = roles
	access_token = key_manager.sign_access_token(subject=str(user.id), claims=claims)
	refresh_token = key_manager.sign_refresh_token(subject=str(user.id), claims={})
	expires_in = settings.JWT_ACCESS_TTL_MINUTES * 60

	# Mark refresh token as active for rotation enforcement
	try:
		rt_claims = decode_jwt(refresh_token, verify_exp=False)
		rt_jti = rt_claims.get("jti")
		if rt_jti:
			key = f"refresh:{rt_jti}"
			redis_client.setex(key, settings.JWT_REFRESH_TTL_DAYS * 86400, "1")
	except Exception:
		# Non-fatal in login path; will be enforced on next refresh
		pass

	return TokenResponse(access_token=access_token, refresh_token=refresh_token, expires_in=expires_in)


@router.post("/refresh")
def refresh(payload: RefreshRequest, redis_client = Depends(get_redis)) -> TokenResponse:
	try:
		claims = decode_jwt(payload.refresh_token, verify_exp=True)
	except Exception:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid refresh token")
	if claims.get("typ") != "refresh":
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="wrong token type")
	subject = claims.get("sub")
	if not subject:
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="missing subject")

	# Enforce single-use refresh tokens (rotation)
	rt_jti = claims.get("jti")
	if not rt_jti:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid refresh token")
	key = f"refresh:{rt_jti}"
	# Atomically get and delete to prevent reuse
	previous = None
	try:
		previous = redis_client.getdel(key)  # type: ignore[attr-defined]
	except Exception:
		# Fallback if Redis server doesn't support GETDEL (pre-6.2)
		pipe = redis_client.pipeline()
		pipe.get(key)
		pipe.delete(key)
		result = pipe.execute()
		previous = result[0]
	if previous is None:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="refresh token already used or unknown")

	# Fetch roles from operators table (need db access)
	from app.dependencies import get_db
	db = next(get_db())
	try:
		user_id = int(subject) if subject.isdigit() else 0
		roles = _fetch_user_roles(user_id, db) if user_id > 0 else []
	finally:
		db.close()
	claims = {}
	if roles:
		claims["roles"] = roles
	access_token = key_manager.sign_access_token(subject=str(subject), claims=claims)
	refresh_token = key_manager.sign_refresh_token(subject=str(subject), claims={})
	# Activate new refresh token
	try:
		new_claims = decode_jwt(refresh_token, verify_exp=False)
		new_jti = new_claims.get("jti")
		if new_jti:
			new_key = f"refresh:{new_jti}"
			redis_client.setex(new_key, settings.JWT_REFRESH_TTL_DAYS * 86400, "1")
	except Exception:
		pass
	expires_in = settings.JWT_ACCESS_TTL_MINUTES * 60
	return TokenResponse(access_token=access_token, refresh_token=refresh_token, expires_in=expires_in)


@router.post("/device/start", response_model=DeviceLoginStartResponse)
def device_login_start(payload: DeviceLoginStartRequest, redis_client = Depends(get_redis)) -> DeviceLoginStartResponse:
	if payload.code_challenge_method not in ("S256", "plain"):
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="unsupported code_challenge_method")
	device_code = f"dl_{int(time.time())}_{hashlib.sha256(str(time.time()).encode()).hexdigest()[:16]}"
	expires_in = settings.DEVICE_LOGIN_EXPIRES_IN_SECONDS
	expires_at = int(time.time()) + expires_in
	state = {
		"status": "pending",
		"code_challenge": payload.code_challenge,
		"method": payload.code_challenge_method,
		"user_hint": str(payload.user_hint) if payload.user_hint else None,
		"expires_at": expires_at,
	}
	key = f"device_login:{device_code}"
	redis_client.setex(key, expires_in, json.dumps(state))
	return DeviceLoginStartResponse(device_code=device_code, expires_in=expires_in, interval=settings.DEVICE_LOGIN_POLL_INTERVAL_SECONDS)


@router.post("/device/authorize", status_code=status.HTTP_204_NO_CONTENT)
def device_login_authorize(payload: DeviceLoginAuthorizeRequest, db: Session = Depends(get_db), redis_client = Depends(get_redis)) -> None:
	key = f"device_login:{payload.device_code}"
	raw = redis_client.get(key)
	if raw is None:
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="invalid or expired device_code")
	state = json.loads(raw)
	if state.get("status") != "pending":
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="invalid state")
	if state.get("expires_at", 0) < int(time.time()):
		redis_client.delete(key)
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="expired")

	user = db.execute(select(User).where(User.email == str(payload.email), User.is_deleted == False)).scalar_one_or_none()  # noqa: E712
	if user is None or not verify_password(payload.password, user.password_hash):
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid credentials")
	if user.totp_secret:
		if not payload.totp_code:
			raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="totp required")
		totp = pyotp.TOTP(user.totp_secret)
		if not totp.verify(payload.totp_code, valid_window=1):
			raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid totp")

	state["status"] = "authorized"
	state["user_id"] = user.id
	ttl = max(0, state["expires_at"] - int(time.time()))
	redis_client.setex(key, ttl, json.dumps(state))
	return None


@router.post("/device/poll", response_model=TokenResponse)
def device_login_poll(payload: DeviceLoginPollRequest, redis_client = Depends(get_redis)) -> TokenResponse:
	key = f"device_login:{payload.device_code}"
	raw = redis_client.get(key)
	if raw is None:
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="invalid or expired device_code")
	state = json.loads(raw)
	now = int(time.time())
	if state.get("expires_at", 0) < now:
		redis_client.delete(key)
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="expired")
	if state.get("status") == "pending":
		raise HTTPException(status_code=status.HTTP_428_PRECONDITION_REQUIRED, detail="authorization_pending")
	if state.get("status") != "authorized":
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="invalid state")

	method = state.get("method")
	challenge = state.get("code_challenge", "")
	if method == "S256":
		chk = hashlib.sha256(payload.code_verifier.encode()).digest()
		verifier_challenge = base64.urlsafe_b64encode(chk).rstrip(b"=").decode("ascii")
	else:
		verifier_challenge = payload.code_verifier
	if verifier_challenge != challenge:
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="invalid code_verifier")

	user_id = state.get("user_id")
	if not user_id:
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="invalid state")
	# One-time consumption
	redis_client.delete(key)

	# Fetch roles from operators table (need db access)
	from app.dependencies import get_db
	db = next(get_db())
	try:
		roles = _fetch_user_roles(user_id, db)
	finally:
		db.close()
	claims = {}
	if roles:
		claims["roles"] = roles
	access_token = key_manager.sign_access_token(subject=str(user_id), claims=claims)
	refresh_token = key_manager.sign_refresh_token(subject=str(user_id), claims={})
	# Track refresh token JTI for rotation
	try:
		rt_claims = decode_jwt(refresh_token, verify_exp=False)
		rt_jti = rt_claims.get("jti")
		if rt_jti:
			rt_key = f"refresh:{rt_jti}"
			redis_client.setex(rt_key, settings.JWT_REFRESH_TTL_DAYS * 86400, "1")
	except Exception:
		pass
	expires_in = settings.JWT_ACCESS_TTL_MINUTES * 60
	return TokenResponse(access_token=access_token, refresh_token=refresh_token, expires_in=expires_in)
