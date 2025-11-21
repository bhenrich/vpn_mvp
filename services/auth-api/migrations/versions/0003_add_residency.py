"""add residency flag to users

Revision ID: 0003_add_residency
Revises: 0002_add_totp_secret
Create Date: 2025-11-13
"""

from __future__ import annotations

from alembic import op
import sqlalchemy as sa


# revision identifiers, used by Alembic.
revision = "0003_add_residency"
down_revision = "0002_add_totp_secret"
branch_labels = None
depends_on = None


def upgrade() -> None:
	op.add_column("users", sa.Column("residency", sa.String(length=16), nullable=True))
	# optional: add index if needed for queries
	# op.create_index("ix_users_residency", "users", ["residency"], unique=False)


def downgrade() -> None:
	# if an index was added, drop it here first
	# op.drop_index("ix_users_residency", table_name="users")
	op.drop_column("users", "residency")


