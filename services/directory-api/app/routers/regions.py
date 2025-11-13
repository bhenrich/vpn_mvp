from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, status, Path, Response
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.security import require_operator, require_admin
from app.models import Region
from app.schemas import RegionCreate, RegionRead, RegionUpdate

router = APIRouter()


@router.get("/", response_model=list[RegionRead])
async def list_regions(db: AsyncSession = Depends(get_db_session)) -> list[Region]:
	result = await db.execute(select(Region).order_by(Region.country_code, Region.city))
	return list(result.scalars().all())


@router.post("/", response_model=RegionRead, status_code=status.HTTP_201_CREATED)
async def create_region(
	payload: RegionCreate,
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> Region:
	existing = await db.execute(
		select(Region).where(Region.country_code == payload.country_code, Region.city == payload.city)
	)
	if existing.scalar_one_or_none() is not None:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="Region already exists")
	region = Region(
		country_code=payload.country_code.upper(),
		city=payload.city,
		status=payload.status,
		latency_score=payload.latency_score,
	)
	db.add(region)
	await db.commit()
	await db.refresh(region)
	return region

@router.put("/{region_id}", response_model=RegionRead)
async def update_region(
	region_id: str = Path(..., description="Region UUID"),
	payload: RegionUpdate = None,
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> Region:
	region = await db.get(Region, region_id)
	if region is None:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Region not found")
	if payload.country_code is not None:
		region.country_code = payload.country_code.upper()
	if payload.city is not None:
		region.city = payload.city
	if payload.status is not None:
		region.status = payload.status
	if payload.latency_score is not None:
		region.latency_score = payload.latency_score
	await db.commit()
	await db.refresh(region)
	return region


@router.delete("/{region_id}", status_code=status.HTTP_204_NO_CONTENT, response_class=Response)
async def delete_region(
	region_id: str,
	_: dict = Depends(require_admin),
	db: AsyncSession = Depends(get_db_session),
) -> Response:
	region = await db.get(Region, region_id)
	if region is None:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Region not found")
	await db.delete(region)
	await db.commit()
	return Response(status_code=status.HTTP_204_NO_CONTENT)


