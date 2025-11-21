import base64
import json
import threading
import time
import uuid
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from typing import Any, Dict, List, Tuple

import jwt
from argon2 import PasswordHasher
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa

from app.config import settings


_password_hasher = PasswordHasher(
	time_cost=settings.ARGON2_TIME_COST,
	memory_cost=settings.ARGON2_MEMORY_COST,
	parallelism=settings.ARGON2_PARALLELISM,
)


def hash_password(password: str) -> str:
	return _password_hasher.hash(password)


def verify_password(password: str, password_hash: str) -> bool:
	try:
		_password_hasher.verify(password_hash, password)
		return True
	except Exception:
		return False


@dataclass(frozen=True)
class KeyPair:
	kid: str
	private_pem: bytes
	public_pem: bytes

	def jwk(self) -> Dict[str, Any]:
		public_key = serialization.load_pem_public_key(self.public_pem)
		public_numbers = public_key.public_numbers()
		e_bytes = public_numbers.e.to_bytes((public_numbers.e.bit_length() + 7) // 8, "big")
		n_bytes = public_numbers.n.to_bytes((public_numbers.n.bit_length() + 7) // 8, "big")
		return {
			"kty": "RSA",
			"kid": self.kid,
			"use": "sig",
			"alg": "RS256",
			"e": base64.urlsafe_b64encode(e_bytes).rstrip(b"=").decode("ascii"),
			"n": base64.urlsafe_b64encode(n_bytes).rstrip(b"=").decode("ascii"),
		}


class KeyManager:
	def __init__(self, rotation_hours: int) -> None:
		self.rotation_seconds = rotation_hours * 3600
		self._lock = threading.RLock()
		self._active: KeyPair
		self._previous: List[Tuple[KeyPair, float]] = []  # (keypair, expires_at)
		self._ensure_key()

	def _generate_keypair(self) -> KeyPair:
		private_key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
		private_pem = private_key.private_bytes(
			encoding=serialization.Encoding.PEM,
			format=serialization.PrivateFormat.PKCS8,
			encryption_algorithm=serialization.NoEncryption(),
		)
		public_pem = private_key.public_key().public_bytes(
			encoding=serialization.Encoding.PEM,
			format=serialization.PublicFormat.SubjectPublicKeyInfo,
		)
		kid = str(uuid.uuid4())
		return KeyPair(kid=kid, private_pem=private_pem, public_pem=public_pem)

	def _ensure_key(self) -> None:
		with self._lock:
			if not hasattr(self, "_active"):
				self._active = self._generate_keypair()
				self._active_created_at = time.time()
			elif time.time() - getattr(self, "_active_created_at", 0.0) > self.rotation_seconds:
				# rotate
				new_key = self._generate_keypair()
				expires_at = time.time() + (settings.JWT_REFRESH_TTL_DAYS * 86400)
				self._previous.append((self._active, expires_at))
				self._active = new_key
				self._active_created_at = time.time()
				# evict expired previous keys
				self._previous = [(k, exp) for (k, exp) in self._previous if exp > time.time()]

	def get_active(self) -> KeyPair:
		self._ensure_key()
		return self._active

	def get_all_public_jwks(self) -> Dict[str, Any]:
		self._ensure_key()
		keys = [self._active.jwk()] + [kp.jwk() for (kp, exp) in self._previous if exp > time.time()]
		return {"keys": keys}

	def sign_access_token(self, subject: str, claims: Dict[str, Any]) -> str:
		now = datetime.now(timezone.utc)
		exp = now + timedelta(minutes=settings.JWT_ACCESS_TTL_MINUTES)
		payload = {
			"iss": settings.JWT_ISSUER,
			"sub": subject,
			"iat": int(now.timestamp()),
			"exp": int(exp.timestamp()),
			"jti": str(uuid.uuid4()),
			**claims,
		}
		key = self.get_active()
		token = jwt.encode(payload, key.private_pem, algorithm="RS256", headers={"kid": key.kid})
		return token

	def sign_refresh_token(self, subject: str, claims: Dict[str, Any]) -> str:
		now = datetime.now(timezone.utc)
		exp = now + timedelta(days=settings.JWT_REFRESH_TTL_DAYS)
		payload = {
			"iss": settings.JWT_ISSUER,
			"sub": subject,
			"iat": int(now.timestamp()),
			"exp": int(exp.timestamp()),
			"jti": str(uuid.uuid4()),
			"typ": "refresh",
			**claims,
		}
		key = self.get_active()
		token = jwt.encode(payload, key.private_pem, algorithm="RS256", headers={"kid": key.kid})
		return token


# Singleton for module-level access during early scaffolding
key_manager = KeyManager(settings.JWKS_ROTATION_HOURS)


def _find_keypair_by_kid(kid: str) -> KeyPair | None:
	active = key_manager.get_active()
	if active.kid == kid:
		return active
	for kp, exp in key_manager._previous:  # type: ignore[attr-defined]
		if kp.kid == kid:
			return kp
	return None


def decode_jwt(token: str, verify_exp: bool = True) -> Dict[str, Any]:
	headers = jwt.get_unverified_header(token)
	kid = headers.get("kid")
	if not kid:
		raise jwt.InvalidTokenError("missing kid")
	keypair = _find_keypair_by_kid(kid)
	if not keypair:
		raise jwt.InvalidTokenError("unknown kid")
	return jwt.decode(
		token,
		keypair.public_pem,
		algorithms=["RS256"],
		options={"verify_exp": verify_exp, "verify_aud": False, "verify_iss": False},
	)
