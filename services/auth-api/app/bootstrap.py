"""Bootstrap script to create seed users on startup"""
from __future__ import annotations

from sqlalchemy import select, text

from app.db import get_db
from app.models import User, Policy
from app.security import hash_password

SEED_USERS = (
	{"email": "test@test.de", "password": "test"},
	{"email": "admin@test.de", "password": "test"},
)


def bootstrap_seed_users() -> None:
	"""Ensure built-in seed accounts exist and have operator privileges."""
	db = next(get_db())
	try:
		for seed in SEED_USERS:
			_ensure_user(db, seed["email"], seed["password"])
	finally:
		db.close()


def _ensure_user(db, email: str, password: str) -> None:
	result = db.execute(select(User).where(User.email == email))
	user = result.scalar_one_or_none()

	if user is None:
		user = User(email=email, password_hash=hash_password(password), is_deleted=False)
		db.add(user)
		db.flush()
		policy = Policy(user_id=user.id, max_active_connections=5)
		db.add(policy)
		db.commit()
		print(f"Created seed user: {email} (ID: {user.id})")
	else:
		user.password_hash = hash_password(password)
		db.commit()
		print(f"Updated seed user: {email} (ID: {user.id})")

	_ensure_operator_row(db, user.id, role="admin" if "admin" in email else "operator")


def _ensure_operator_row(db, user_id: int, role: str) -> None:
	"""Ensure a row exists in operators table for the given user."""
	try:
		db.execute(
			text(
				"""
				INSERT INTO operators (user_id, role)
				VALUES (:user_id, :role)
				ON CONFLICT (user_id) DO UPDATE SET role = EXCLUDED.role
				"""
			),
			{"user_id": user_id, "role": role},
		)
		db.commit()
		print(f"Granted {role} role to user {user_id} in operators table")
	except Exception as exc:  # pragma: no cover - table may not exist yet
		db.rollback()
		print(f"Warning: Could not ensure operator row for user {user_id}: {exc}")

