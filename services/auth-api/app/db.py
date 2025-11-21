from typing import Generator

from sqlalchemy import create_engine
from sqlalchemy.orm import Session, declarative_base, sessionmaker

from app.config import settings


def _effective_database_url(url: str) -> str:
	"""
	Ensure SQLAlchemy uses psycopg (v3) driver when a plain 'postgresql://' URL is provided.
	"""
	if url.startswith("postgresql://"):
		return "postgresql+psycopg://" + url[len("postgresql://") :]
	return url

engine = create_engine(_effective_database_url(settings.DATABASE_URL), pool_pre_ping=True, future=True)
SessionLocal = sessionmaker(bind=engine, autoflush=False, autocommit=False, future=True)
Base = declarative_base()


def get_db() -> Generator[Session, None, None]:
	db = SessionLocal()
	try:
		yield db
	finally:
		db.close()


