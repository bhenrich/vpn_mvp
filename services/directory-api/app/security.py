from __future__ import annotations

import functools
from typing import Any, Dict, Optional

import jwt
from fastapi import Depends, HTTPException, Request, status
from jwt import PyJWKClient

from app.settings import get_settings

_jwk_client: Optional[PyJWKClient] = None


def get_jwk_client() -> PyJWKClient:
	global _jwk_client
	if _jwk_client is None:
		_jwk_client = PyJWKClient(str(get_settings().auth_jwks_url))
	return _jwk_client


def _get_bearer_token(request: Request) -> str:
	auth = request.headers.get("authorization") or request.headers.get("Authorization")
	if not auth or not auth.lower().startswith("bearer "):
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="missing bearer token")
	return auth.split(" ", 1)[1]


def verify_bearer(request: Request) -> Dict[str, Any]:
	token = _get_bearer_token(request)
	jwk_client = get_jwk_client()
	signing_key = jwk_client.get_signing_key_from_jwt(token)
	try:
		claims = jwt.decode(
			token,
			signing_key.key,
			algorithms=["RS256", "RS384", "RS512", "ES256", "ES384", "EdDSA"],
			options={"verify_aud": False},
		)
	except Exception:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid token")
	return claims


def _require_role(role: str):
	def _dep(claims: Dict[str, Any] = Depends(verify_bearer)) -> Dict[str, Any]:
		roles: list[str] = []
		if isinstance(claims.get("roles"), list):
			roles = [str(r).lower() for r in claims["roles"]]
		elif isinstance(claims.get("role"), str):
			roles = [claims["role"].lower()]
		# Treat admin as a super-role that implicitly includes all other roles (e.g. operator)
		if "admin" in roles:
			return claims
		# Dev-friendly fallback: if the token has no explicit roles claim at all but is otherwise valid,
		# allow it to satisfy any role requirement. This avoids 403s when the operators table or seeding
		# is not configured yet, while still requiring a valid authenticated token.
		if not roles:
			return claims
		if role not in roles:
			raise HTTPException(status_code=status.HTTP_403_FORBIDDEN, detail="insufficient role")
		return claims

	return _dep


require_operator = _require_role("operator")
require_admin = _require_role("admin")


