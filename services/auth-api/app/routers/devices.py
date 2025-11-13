import base64

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.orm import Session

from app.dependencies import get_current_user_id, get_db
from app.models import Device
from app.schemas import DeviceRegisterRequest, DeviceResponse

router = APIRouter(prefix="/devices", tags=["devices"])


@router.post("/register", response_model=DeviceResponse, status_code=status.HTTP_201_CREATED)
def register_device(
	payload: DeviceRegisterRequest,
	user_id: int = Depends(get_current_user_id),
	db: Session = Depends(get_db),
) -> DeviceResponse:
	try:
		pubkey_bytes = base64.b64decode(payload.device_public_key_b64, validate=True)
	except Exception:
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="invalid base64 public key")
	if len(pubkey_bytes) not in (32, 64):
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="invalid public key length")

	existing = db.execute(select(Device).where(Device.device_public_key == pubkey_bytes)).scalar_one_or_none()
	if existing is not None:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="device already registered")

	device = Device(user_id=user_id, device_public_key=pubkey_bytes, platform=payload.platform)
	db.add(device)
	db.commit()
	db.refresh(device)
	return DeviceResponse(id=device.id)


