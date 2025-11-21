from __future__ import annotations

from alembic import op
import sqlalchemy as sa

# revision identifiers, used by Alembic.
revision = "0001_initial"
down_revision = None
branch_labels = None
depends_on = None


def upgrade() -> None:
	op.create_table(
		"users",
		sa.Column("id", sa.Integer(), primary_key=True, autoincrement=True),
		sa.Column("email", sa.String(length=320), nullable=False),
		sa.Column("password_hash", sa.String(length=512), nullable=False),
		sa.Column("created_at", sa.TIMESTAMP(timezone=True), server_default=sa.func.now(), nullable=False),
		sa.Column("tos_version", sa.String(length=32), nullable=True),
		sa.Column("gdpr_consent_at", sa.TIMESTAMP(timezone=True), nullable=True),
		sa.Column("is_deleted", sa.Boolean(), nullable=False, server_default=sa.text("false")),
		sa.UniqueConstraint("email", name="uq_users_email"),
	)
	op.create_index("ix_users_email", "users", ["email"])

	op.create_table(
		"devices",
		sa.Column("id", sa.Integer(), primary_key=True, autoincrement=True),
		sa.Column("user_id", sa.Integer(), sa.ForeignKey("users.id", ondelete="CASCADE"), nullable=False),
		sa.Column("device_public_key", sa.LargeBinary(length=64), nullable=False),
		sa.Column("platform", sa.String(length=32), nullable=False),
		sa.Column("created_at", sa.TIMESTAMP(timezone=True), server_default=sa.func.now(), nullable=False),
		sa.Column("last_seen_at", sa.TIMESTAMP(timezone=True), nullable=True),
		sa.UniqueConstraint("device_public_key", name="uq_devices_public_key"),
	)
	op.create_index("ix_devices_user_id", "devices", ["user_id"])

	op.create_table(
		"sessions",
		sa.Column("id", sa.Integer(), primary_key=True, autoincrement=True),
		sa.Column("user_id", sa.Integer(), sa.ForeignKey("users.id", ondelete="CASCADE"), nullable=False),
		sa.Column("device_id", sa.Integer(), sa.ForeignKey("devices.id", ondelete="CASCADE"), nullable=False),
		sa.Column("issued_at", sa.TIMESTAMP(timezone=True), server_default=sa.func.now(), nullable=False),
		sa.Column("expires_at", sa.TIMESTAMP(timezone=True), nullable=False),
		sa.Column("revoked_at", sa.TIMESTAMP(timezone=True), nullable=True),
		sa.Column("session_jti", sa.String(length=64), nullable=False),
		sa.UniqueConstraint("session_jti", name="uq_sessions_session_jti"),
	)
	op.create_index("ix_sessions_user_id", "sessions", ["user_id"])
	op.create_index("ix_sessions_device_id", "sessions", ["device_id"])
	op.create_index("ix_sessions_user_device", "sessions", ["user_id", "device_id"])

	op.create_table(
		"policies",
		sa.Column("id", sa.Integer(), primary_key=True, autoincrement=True),
		sa.Column("user_id", sa.Integer(), sa.ForeignKey("users.id", ondelete="CASCADE"), nullable=False),
		sa.Column("max_active_connections", sa.Integer(), nullable=False, server_default=sa.text("5")),
		sa.UniqueConstraint("user_id", name="uq_policies_user_id"),
	)
	op.create_index("ix_policies_user_id", "policies", ["user_id"])


def downgrade() -> None:
	op.drop_index("ix_policies_user_id", table_name="policies")
	op.drop_table("policies")

	op.drop_index("ix_sessions_user_device", table_name="sessions")
	op.drop_index("ix_sessions_device_id", table_name="sessions")
	op.drop_index("ix_sessions_user_id", table_name="sessions")
	op.drop_table("sessions")

	op.drop_index("ix_devices_user_id", table_name="devices")
	op.drop_table("devices")

	op.drop_index("ix_users_email", table_name="users")
	op.drop_table("users")


