from __future__ import annotations

from typing import Optional, AsyncGenerator
from redis.asyncio import Redis

from app.settings import get_settings

_redis: Optional[Redis] = None


async def init_redis() -> None:
	global _redis
	if _redis is None:
		_redis = Redis.from_url(str(get_settings().redis_url), decode_responses=True)
		# light-touch connectivity check
		await _redis.ping()


def get_redis_client() -> Redis:
	if _redis is None:
		raise RuntimeError("Redis client not initialized")
	return _redis


async def close_redis() -> None:
	global _redis
	if _redis is not None:
		await _redis.aclose()
		_redis = None


