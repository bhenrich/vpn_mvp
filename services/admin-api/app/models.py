from __future__ import annotations

from datetime import datetime
from typing import Optional

from sqlalchemy import BigInteger, Boolean, CheckConstraint, Column, DateTime, Enum, Integer, JSON, String, UniqueConstraint, text
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column


class Base(DeclarativeBase):
	pass


class Operator(Base):
	__tablename__ = "operators"

	id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
	user_id: Mapped[int] = mapped_column(BigInteger, nullable=False, unique=True, index=True)
	role: Mapped[str] = mapped_column(Enum("admin", "operator", "read_only", name="operator_role"), nullable=False, index=True, default="operator")
	created_at: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False, server_default=text("CURRENT_TIMESTAMP"))


class FeatureFlag(Base):
	__tablename__ = "feature_flags"

	key: Mapped[str] = mapped_column(String(128), primary_key=True)
	enabled: Mapped[bool] = mapped_column(Boolean, nullable=False, default=False)
	description: Mapped[Optional[str]] = mapped_column(String(512), nullable=True)
	rollout_percentage: Mapped[int] = mapped_column(Integer, nullable=False, default=100)

	__table_args__ = (
		CheckConstraint("rollout_percentage >= 0 AND rollout_percentage <= 100", name="ck_rollout_pct_range"),
	)


class AuditEvent(Base):
	__tablename__ = "audit_events"

	id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
	time: Mapped[datetime] = mapped_column(DateTime(timezone=True), nullable=False, server_default=text("CURRENT_TIMESTAMP"))
	type: Mapped[str] = mapped_column(String(64), nullable=False, index=True)
	subject_hash: Mapped[str] = mapped_column(String(128), nullable=False, index=True)
	payload_redacted: Mapped[dict] = mapped_column(JSON, nullable=False, default=dict)
	retention_tag: Mapped[str] = mapped_column(String(32), nullable=False, default="short")

	__table_args__ = (
		UniqueConstraint("id", name="uq_audit_id"),
	)



