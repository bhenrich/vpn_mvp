from __future__ import annotations

import uuid
from datetime import datetime, timezone

from sqlalchemy import String, ForeignKey, Enum, DateTime, UniqueConstraint, JSON
from sqlalchemy.types import TypeDecorator


class UuidCompat(TypeDecorator):
	impl = String
	cache_ok = True

	def load_dialect_impl(self, dialect):
		if dialect.name == "postgresql":
			from sqlalchemy.dialects.postgresql import UUID as PG_UUID  # type: ignore
			return dialect.type_descriptor(PG_UUID(as_uuid=True))
		return dialect.type_descriptor(String(36))

	def process_bind_param(self, value, dialect):
		if value is None:
			return None
		if dialect.name == "postgresql":
			return value
		# store as string in sqlite/others
		return str(value)

	def process_result_value(self, value, dialect):
		if value is None:
			return None
		# always return uuid.UUID
		return uuid.UUID(str(value))


class InetCompat(TypeDecorator):
	impl = String
	cache_ok = True

	def load_dialect_impl(self, dialect):
		if dialect.name == "postgresql":
			from sqlalchemy.dialects.postgresql import INET as PG_INET  # type: ignore
			return dialect.type_descriptor(PG_INET())
		# Max IPv6 textual length
		return dialect.type_descriptor(String(45))


class JsonCompat(TypeDecorator):
	impl = JSON
	cache_ok = True

	def load_dialect_impl(self, dialect):
		if dialect.name == "postgresql":
			from sqlalchemy.dialects.postgresql import JSONB as PG_JSONB  # type: ignore
			return dialect.type_descriptor(PG_JSONB())
		return dialect.type_descriptor(JSON())
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship


class Base(DeclarativeBase):
	pass


class Region(Base):
	__tablename__ = "regions"

	id: Mapped[uuid.UUID] = mapped_column(UuidCompat(), primary_key=True, default=uuid.uuid4)
	country_code: Mapped[str] = mapped_column(String(2), nullable=False)
	city: Mapped[str] = mapped_column(String(128), nullable=False)
	status: Mapped[str] = mapped_column(String(16), default="active", nullable=False)
	latency_score: Mapped[float] = mapped_column(nullable=True)

	nodes: Mapped[list[Node]] = relationship(back_populates="region", cascade="all, delete-orphan")

	__table_args__ = (UniqueConstraint("country_code", "city", name="uq_region_country_city"),)


class Node(Base):
	__tablename__ = "nodes"

	id: Mapped[uuid.UUID] = mapped_column(UuidCompat(), primary_key=True, default=uuid.uuid4)
	public_key: Mapped[str] = mapped_column(String(255), unique=True, nullable=False)

	region_id: Mapped[uuid.UUID | None] = mapped_column(UuidCompat(), ForeignKey("regions.id"), nullable=True)
	region: Mapped[Region | None] = relationship(back_populates="nodes")

	internal_wg_ip: Mapped[str | None] = mapped_column(InetCompat(), nullable=True)
	egress_ips: Mapped[list[str] | None] = mapped_column(JsonCompat(), nullable=True)

	capabilities: Mapped[dict | None] = mapped_column(JsonCompat(), nullable=True)
	status: Mapped[str] = mapped_column(String(16), default="unknown", nullable=False)

	agent_version: Mapped[str | None] = mapped_column(String(64), nullable=True)
	wg_rs_version: Mapped[str | None] = mapped_column(String(64), nullable=True)

	last_seen_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), default=None)

	def touch(self) -> None:
		self.last_seen_at = datetime.now(timezone.utc)


