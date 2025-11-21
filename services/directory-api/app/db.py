from collections.abc import AsyncGenerator
import asyncio
from sqlalchemy.ext.asyncio import AsyncEngine, AsyncSession, async_sessionmaker, create_async_engine
from sqlalchemy import text
from fastapi import Depends

from app.settings import get_settings

_engines: dict[int, AsyncEngine] = {}
_session_factories: dict[int, async_sessionmaker[AsyncSession]] = {}


def _effective_database_url(url: str) -> str:
	# Force asyncpg driver when a plain 'postgresql://' URL is provided (e.g., from env)
	return "postgresql+asyncpg://" + url[len("postgresql://") :] if url.startswith("postgresql://") else url


def _current_loop_id() -> int:
	try:
		return id(asyncio.get_running_loop())
	except RuntimeError:
		# Fallback for contexts where no running loop is set
		return id(asyncio.get_event_loop())


def get_engine() -> AsyncEngine:
	loop_id = _current_loop_id()
	engine = _engines.get(loop_id)
	if engine is None:
		dsn = _effective_database_url(str(get_settings().database_url))
		engine = create_async_engine(dsn, echo=False, future=True)
		_engines[loop_id] = engine
		_session_factories[loop_id] = async_sessionmaker(bind=engine, expire_on_commit=False, autoflush=False)
	return engine


def get_session_factory() -> async_sessionmaker[AsyncSession]:
	loop_id = _current_loop_id()
	session_factory = _session_factories.get(loop_id)
	if session_factory is None:
		get_engine()
		session_factory = _session_factories[loop_id]
	return session_factory


async def get_db_session() -> AsyncGenerator[AsyncSession, None]:
	session_factory = get_session_factory()
	async with session_factory() as session:
		yield session


async def init_db() -> None:
	# Run a trivial statement to force-initialize the engine and validate connectivity
	engine = get_engine()
	async with engine.begin() as conn:
		await conn.run_sync(lambda sync_conn: None)
		# Create tables for dev usage (production uses Alembic migrations)
		from app.models import Base  # local import to avoid circular dependency
		await conn.run_sync(Base.metadata.create_all)


