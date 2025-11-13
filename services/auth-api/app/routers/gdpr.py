from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.orm import Session

from app.dependencies import get_current_user_id, get_db
from app.models import Device, Session as DbSession, User

router = APIRouter(prefix="/gdpr", tags=["gdpr"])


@router.post("/export")
def export(user_id: int = Depends(get_current_user_id), db: Session = Depends(get_db)) -> dict:
	user = db.execute(select(User).where(User.id == user_id)).scalar_one_or_none()
	if user is None:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="user not found")
	devices = db.execute(select(Device).where(Device.user_id == user_id)).scalars().all()
	return {
		"user": {
			"id": user.id,
			"email": user.email,
			"created_at": user.created_at.isoformat() if user.created_at else None,
			"tos_version": user.tos_version,
			"gdpr_consent_at": user.gdpr_consent_at.isoformat() if user.gdpr_consent_at else None,
			"is_deleted": user.is_deleted,
		},
		"devices": [
			{
				"id": d.id,
				"platform": d.platform,
				"created_at": d.created_at.isoformat() if d.created_at else None,
				"last_seen_at": d.last_seen_at.isoformat() if d.last_seen_at else None,
			}
			for d in devices
		],
	}


@router.post("/delete", status_code=status.HTTP_202_ACCEPTED)
def delete(user_id: int = Depends(get_current_user_id), db: Session = Depends(get_db)) -> dict:
	user = db.execute(select(User).where(User.id == user_id)).scalar_one_or_none()
	if user is None:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="user not found")
	# Hard-delete user and all associated records to fulfill GDPR erase.
	# Explicitly delete dependent rows for clarity; FKs also set ON DELETE CASCADE.
	db.query(DbSession).filter(DbSession.user_id == user.id).delete(synchronize_session=False)
	db.query(Device).filter(Device.user_id == user.id).delete(synchronize_session=False)
	db.delete(user)
	db.commit()
	return {"status": "deleted", "at": datetime.now(timezone.utc).isoformat()}


