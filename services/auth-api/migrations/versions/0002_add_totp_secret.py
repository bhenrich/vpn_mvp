from __future__ import annotations

from alembic import op
import sqlalchemy as sa

# revision identifiers, used by Alembic.
revision = "0002_add_totp_secret"
down_revision = "0001_initial"
branch_labels = None
depends_on = None


def upgrade() -> None:
	op.add_column("users", sa.Column("totp_secret", sa.String(length=64), nullable=True))


def downgrade() -> None:
	op.drop_column("users", "totp_secret")


