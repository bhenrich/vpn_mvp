from typing import Optional

import redis
from fastapi import Depends, Header, HTTPException, status
from sqlalchemy import select
from sqlalchemy.orm import Session

from app.config import settings
from app.db import get_db as _get_db
from app.models import User
from app.security import decode_jwt


def get_db() -> Session:
	yield from _get_db()


_redis_client: Optional[redis.Redis] = None


def get_redis() -> redis.Redis:
	global _redis_client
	if _redis_client is None:
		_redis_client = redis.Redis.from_url(settings.REDIS_URL, decode_responses=True)
	return _redis_client


def get_current_user_id(
	authorization: str = Header(alias="Authorization"),
	db: Session = Depends(get_db),
) -> int:
	if not authorization or not authorization.lower().startswith("bearer "):
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="missing bearer token")
	token = authorization.split(" ", 1)[1]
	try:
		claims = decode_jwt(token, verify_exp=True)
	except Exception:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid token")
	sub = claims.get("sub")
	if sub is None:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid token subject")
	# Enforce that the user exists and is not deleted at token consumption time
	try:
		user_id = int(sub)
	except Exception:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="invalid token subject")
	user = db.execute(select(User).where(User.id == user_id, User.is_deleted == False)).scalar_one_or_none()  # noqa: E712
	if user is None:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="account unavailable")
	return user_id


