from collections.abc import AsyncGenerator
from sqlalchemy.ext.asyncio import AsyncEngine, AsyncSession, async_sessionmaker, create_async_engine

from app.settings import get_settings

_engine: AsyncEngine | None = None
_session_factory: async_sessionmaker[AsyncSession] | None = None


def _effective_database_url(url: str) -> str:
	# Force asyncpg driver when a plain 'postgresql://' URL is provided (e.g., from env)
	return "postgresql+asyncpg://" + url[len("postgresql://") :] if url.startswith("postgresql://") else url


def get_engine() -> AsyncEngine:
	global _engine, _session_factory
	if _engine is None:
		dsn = _effective_database_url(str(get_settings().database_url))
		_engine = create_async_engine(dsn, echo=False, future=True)
		_session_factory = async_sessionmaker(bind=_engine, expire_on_commit=False, autoflush=False)
	return _engine


def get_session_factory() -> async_sessionmaker[AsyncSession]:
	global _session_factory
	if _session_factory is None:
		get_engine()
	assert _session_factory is not None
	return _session_factory


async def get_db_session() -> AsyncGenerator[AsyncSession, None]:
	session_factory = get_session_factory()
	async with session_factory() as session:
		yield session


async def init_db() -> None:
	engine = get_engine()
	async with engine.begin() as conn:
		# Create tables for dev usage (production uses Alembic migrations)
		from app.models import Base  # local import to avoid circular dependency
		await conn.run_sync(Base.metadata.create_all)



