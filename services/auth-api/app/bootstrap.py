"""Bootstrap script to create test user on startup"""
from __future__ import annotations

import requests
from sqlalchemy import select

from app.config import settings
from app.db import get_db
from app.models import User, Policy
from app.security import hash_password


def bootstrap_test_user() -> None:
	"""Create test user if it doesn't exist and make them an operator"""
	email = "test@test.de"
	password = "test"
	
	db = next(get_db())
	try:
		# Check if user exists
		result = db.execute(select(User).where(User.email == email))
		user = result.scalar_one_or_none()
		
		if user is None:
			# Create new user
			user = User(
				email=email,
				password_hash=hash_password(password),
				is_deleted=False,
			)
			db.add(user)
			db.flush()  # Get the user ID
			
			# Create default policy
			policy = Policy(
				user_id=user.id,
				max_active_connections=5,
			)
			db.add(policy)
			
			db.commit()
			print(f"Created test user: {email} (ID: {user.id})")
			
			# Make user an operator via admin-api
			_make_user_operator(user.id)
		else:
			# Update existing user's email if it's wrong, or reset password
			if user.email != email:
				user.email = email
			user.password_hash = hash_password(password)
			db.commit()
			print(f"Updated test user: {email} (ID: {user.id})")
			
			# Ensure user is an operator
			_make_user_operator(user.id)
	finally:
		db.close()


def _make_user_operator(user_id: int) -> None:
	"""Make the user an operator in admin-api"""
	try:
		admin_api_url = getattr(settings, "ADMIN_API_URL", "http://admin-api:8082")
		# Check if operator exists
		resp = requests.get(
			f"{admin_api_url}/operators",
			timeout=5
		)
		if resp.status_code == 200:
			operators = resp.json()
			if not any(op.get("user_id") == user_id for op in operators):
				# Create operator
				resp = requests.post(
					f"{admin_api_url}/operators",
					json={"user_id": user_id, "role": "operator"},
					timeout=5
				)
				if resp.status_code in (200, 201):
					print(f"Made user {user_id} an operator")
				else:
					print(f"Warning: Failed to make user {user_id} an operator: {resp.status_code}")
	except Exception as e:
		# Non-fatal - admin-api might not be ready yet
		print(f"Warning: Could not make user {user_id} an operator: {e}")

