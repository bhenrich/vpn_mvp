from __future__ import annotations

from datetime import datetime
from typing import Optional

from sqlalchemy import TIMESTAMP, Boolean, ForeignKey, Index, Integer, LargeBinary, String, UniqueConstraint, func
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.db import Base


class User(Base):
	__tablename__ = "users"

	id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
	email: Mapped[str] = mapped_column(String(320), unique=True, nullable=False, index=True)
	password_hash: Mapped[str] = mapped_column(String(512), nullable=False)
	totp_secret: Mapped[Optional[str]] = mapped_column(String(64), nullable=True)
	created_at: Mapped[datetime] = mapped_column(TIMESTAMP(timezone=True), server_default=func.now(), nullable=False)
	tos_version: Mapped[Optional[str]] = mapped_column(String(32), nullable=True)
	gdpr_consent_at: Mapped[Optional[datetime]] = mapped_column(TIMESTAMP(timezone=True), nullable=True)
	is_deleted: Mapped[bool] = mapped_column(Boolean, nullable=False, server_default="false")
	residency: Mapped[Optional[str]] = mapped_column(String(16), nullable=True)

	devices: Mapped[list["Device"]] = relationship(back_populates="user", cascade="all, delete-orphan")
	sessions: Mapped[list["Session"]] = relationship(back_populates="user", cascade="all, delete-orphan")
	policy: Mapped[Optional["Policy"]] = relationship(back_populates="user", uselist=False, cascade="all, delete-orphan")


class Device(Base):
	__tablename__ = "devices"

	id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
	user_id: Mapped[int] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), nullable=False, index=True)
	device_public_key: Mapped[bytes] = mapped_column(LargeBinary(64), nullable=False, unique=True)
	platform: Mapped[str] = mapped_column(String(32), nullable=False)
	created_at: Mapped[datetime] = mapped_column(TIMESTAMP(timezone=True), server_default=func.now(), nullable=False)
	last_seen_at: Mapped[Optional[datetime]] = mapped_column(TIMESTAMP(timezone=True), nullable=True)

	user: Mapped["User"] = relationship(back_populates="devices")
	sessions: Mapped[list["Session"]] = relationship(back_populates="device", cascade="all, delete-orphan")


class Session(Base):
	__tablename__ = "sessions"
	__table_args__ = (
		UniqueConstraint("session_jti", name="uq_sessions_session_jti"),
		Index("ix_sessions_user_device", "user_id", "device_id"),
	)

	id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
	user_id: Mapped[int] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), nullable=False, index=True)
	device_id: Mapped[int] = mapped_column(ForeignKey("devices.id", ondelete="CASCADE"), nullable=False, index=True)
	issued_at: Mapped[datetime] = mapped_column(TIMESTAMP(timezone=True), server_default=func.now(), nullable=False)
	expires_at: Mapped[datetime] = mapped_column(TIMESTAMP(timezone=True), nullable=False)
	revoked_at: Mapped[Optional[datetime]] = mapped_column(TIMESTAMP(timezone=True), nullable=True)
	session_jti: Mapped[str] = mapped_column(String(64), nullable=False)

	user: Mapped["User"] = relationship(back_populates="sessions")
	device: Mapped["Device"] = relationship(back_populates="sessions")


class Policy(Base):
	__tablename__ = "policies"
	__table_args__ = (UniqueConstraint("user_id", name="uq_policies_user_id"),)

	id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
	user_id: Mapped[int] = mapped_column(ForeignKey("users.id", ondelete="CASCADE"), nullable=False, index=True)
	max_active_connections: Mapped[int] = mapped_column(Integer, nullable=False, server_default="5")

	user: Mapped["User"] = relationship(back_populates="policy")


