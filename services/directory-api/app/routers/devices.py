from __future__ import annotations

from typing import Any
import uuid

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.models import ClientDevice
from app.schemas import DeviceRegisterRequest, DeviceRead
from app.security import verify_bearer
from app.services.client_devices import ensure_client_ipv4

router = APIRouter()


def _claims_user_id(claims: dict[str, Any]) -> str:
	sub = str(claims.get("sub") or "").strip()
	if not sub:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="missing subject claim")
	return sub


async def _find_device_for_user(
	db: AsyncSession,
	device_id: uuid.UUID,
	user_id: str,
) -> ClientDevice:
	device = await db.get(ClientDevice, device_id)
	if device is None or device.user_id != user_id:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="device not found")
	return device


@router.post("/register", response_model=DeviceRead, status_code=status.HTTP_201_CREATED)
async def register_or_update_device(
	payload: DeviceRegisterRequest,
	claims: dict[str, Any] = Depends(verify_bearer),
	db: AsyncSession = Depends(get_db_session),
) -> ClientDevice:
	user_id = _claims_user_id(claims)

	device: ClientDevice | None = None
	if payload.device_id is not None:
		device = await _find_device_for_user(db, payload.device_id, user_id)
	else:
		existing = await db.execute(select(ClientDevice).where(ClientDevice.wg_pubkey == payload.wg_public_key))
		existing_device = existing.scalar_one_or_none()
		if existing_device is not None:
			# Existing device re-registering with same public key; ensure ownership
			if existing_device.user_id != user_id:
				raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="public key already registered")
			device = existing_device

	if device is None:
		device = ClientDevice(
			user_id=user_id,
			device_name=payload.device_name or "Unnamed device",
			platform=payload.platform,
			wg_pubkey=payload.wg_public_key,
			status="registered",
		)
		db.add(device)
	else:
		if payload.device_name:
			device.device_name = payload.device_name
		device.platform = payload.platform
		device.wg_pubkey = payload.wg_public_key
		device.status = "registered"

	await ensure_client_ipv4(db, device)
	device.touch()

	await db.commit()
	await db.refresh(device)
	return device



