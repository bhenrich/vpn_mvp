from __future__ import annotations

import time
from typing import Any, Dict, Optional

import httpx
import jwt
from fastapi import Depends, HTTPException, Request, status
from jwt import PyJWKClient
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.models import Operator
from app.settings import get_settings


_jwks_client: PyJWKClient | None = None
_jwks_last_refresh: float = 0.0
_jwks_cache_seconds: int = 300


def _get_jwks_client() -> PyJWKClient:
	global _jwks_client, _jwks_last_refresh
	settings = get_settings()
	if _jwks_client is None or (time.time() - _jwks_last_refresh) > _jwks_cache_seconds:
		# Probe JWKS to fail early if unreachable
		try:
			with httpx.Client(timeout=5.0) as client:
				resp = client.get(str(settings.auth_jwks_url))
				resp.raise_for_status()
		except Exception as exc:
			# Create client anyway; decode will retry fetch per kid
			pass
		_jwks_client = PyJWKClient(str(settings.auth_jwks_url))
		_jwks_last_refresh = time.time()
	return _jwks_client  # type: ignore[return-value]


async def _decode_bearer_token(request: Request) -> Dict[str, Any]:
	authz = request.headers.get("authorization") or request.headers.get("Authorization")
	if not authz or not authz.lower().startswith("bearer "):
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="missing bearer token")
	token = authz.split(" ", 1)[1].strip()
	try:
		unverified = jwt.get_unverified_header(token)
		kid = unverified.get("kid")
		if not kid:
			raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid token header")
		jwks_client = _get_jwks_client()
		signing_key = jwks_client.get_signing_key_from_jwt(token).key
		claims = jwt.decode(
			token,
			signing_key,
			algorithms=["RS256"],
			options={"verify_aud": False, "verify_iss": False},
		)
		return claims
	except HTTPException:
		raise
	except Exception:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid or expired token")


_ROLE_RANK = {"read_only": 0, "operator": 1, "admin": 2}


def _has_required_role(actual: str, required: str) -> bool:
	return _ROLE_RANK.get(actual, -1) >= _ROLE_RANK.get(required, 99)


async def require_operator(
	request: Request,
	required_role: str = "operator",
	db: AsyncSession = Depends(get_db_session),
) -> Dict[str, Any]:
	claims = await _decode_bearer_token(request)
	subject = claims.get("sub")
	if not subject:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="missing subject")
	try:
		user_id = int(str(subject))
	except Exception:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid subject")

	result = await db.execute(select(Operator).where(Operator.user_id == user_id))
	operator: Operator | None = result.scalar_one_or_none()
	if operator is None:
		raise HTTPException(status_code=status.HTTP_403_FORBIDDEN, detail="not an operator")
	if not _has_required_role(operator.role, required_role):
		raise HTTPException(status_code=status.HTTP_403_FORBIDDEN, detail="insufficient role")
	return claims


async def require_admin(
	request: Request,
	db: AsyncSession = Depends(get_db_session),
) -> Dict[str, Any]:
	return await require_operator(request=request, required_role="admin", db=db)



