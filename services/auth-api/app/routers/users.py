from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.orm import Session

from app.dependencies import get_db
from app.models import Policy, User
from app.schemas import UserRegisterRequest, UserResponse
from app.security import hash_password

router = APIRouter(prefix="/users", tags=["users"])


@router.post("/register", response_model=UserResponse, status_code=status.HTTP_201_CREATED)
def register(payload: UserRegisterRequest, db: Session = Depends(get_db)) -> UserResponse:
	existing = db.execute(select(User).where(User.email == payload.email)).scalar_one_or_none()
	if existing is not None:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="email already registered")

	user = User(
		email=str(payload.email),
		password_hash=hash_password(payload.password),
		tos_version=payload.tos_version,
		gdpr_consent_at=datetime.now(timezone.utc) if payload.consent else None,
		residency=payload.residency,
	)
	db.add(user)
	db.flush()

	# Ensure a default policy row exists
	policy = Policy(user_id=user.id, max_active_connections=5)
	db.add(policy)

	db.commit()
	db.refresh(user)
	return UserResponse.model_validate(user)


