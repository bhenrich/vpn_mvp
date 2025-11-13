from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import delete, select, update
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.models import FeatureFlag
from app.schemas import FeatureFlagCreateRequest, FeatureFlagResponse, FeatureFlagUpdateRequest
from app.security import require_admin, require_operator

router = APIRouter(prefix="/flags", tags=["feature-flags"])


@router.get("/", response_model=list[FeatureFlagResponse])
async def list_flags(
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> list[FeatureFlagResponse]:
	result = await db.execute(select(FeatureFlag).order_by(FeatureFlag.key.asc()))
	return [FeatureFlagResponse.model_validate(f) for f in result.scalars().all()]


@router.post("/", response_model=FeatureFlagResponse, status_code=status.HTTP_201_CREATED)
async def create_flag(
	payload: FeatureFlagCreateRequest,
	_: dict = Depends(require_admin),
	db: AsyncSession = Depends(get_db_session),
) -> FeatureFlagResponse:
	existing = await db.execute(select(FeatureFlag).where(FeatureFlag.key == payload.key))
	if existing.scalar_one_or_none() is not None:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="flag already exists")
	flag = FeatureFlag(
		key=payload.key,
		enabled=payload.enabled,
		description=payload.description,
		rollout_percentage=payload.rollout_percentage,
	)
	db.add(flag)
	await db.commit()
	await db.refresh(flag)
	return FeatureFlagResponse.model_validate(flag)


@router.put("/{key}", response_model=FeatureFlagResponse)
async def update_flag(
	key: str,
	payload: FeatureFlagUpdateRequest,
	_: dict = Depends(require_admin),
	db: AsyncSession = Depends(get_db_session),
) -> FeatureFlagResponse:
	result = await db.execute(select(FeatureFlag).where(FeatureFlag.key == key))
	flag = result.scalar_one_or_none()
	if flag is None:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="not found")
	if payload.enabled is not None:
		flag.enabled = payload.enabled
	if payload.description is not None:
		flag.description = payload.description
	if payload.rollout_percentage is not None:
		flag.rollout_percentage = payload.rollout_percentage
	await db.commit()
	await db.refresh(flag)
	return FeatureFlagResponse.model_validate(flag)


@router.delete("/{key}", status_code=status.HTTP_200_OK)
async def delete_flag(
	key: str,
	_: dict = Depends(require_admin),
	db: AsyncSession = Depends(get_db_session),
) -> None:
	await db.execute(delete(FeatureFlag).where(FeatureFlag.key == key))
	await db.commit()
	return {"status": "deleted"}



