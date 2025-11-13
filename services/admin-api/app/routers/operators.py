from __future__ import annotations

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import delete, select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.models import Operator
from app.schemas import OperatorCreateRequest, OperatorResponse
from app.security import require_operator, require_admin

router = APIRouter(prefix="/operators", tags=["operators"])


@router.get("/", response_model=list[OperatorResponse])
async def list_operators(
	_: dict = Depends(lambda r=Depends(require_operator): r),
	db: AsyncSession = Depends(get_db_session),
) -> list[OperatorResponse]:
	result = await db.execute(select(Operator).order_by(Operator.created_at.desc()))
	return [OperatorResponse.model_validate(o) for o in result.scalars().all()]


@router.post("/", response_model=OperatorResponse, status_code=status.HTTP_201_CREATED)
async def create_operator(
	payload: OperatorCreateRequest,
	_: dict = Depends(require_admin),
	db: AsyncSession = Depends(get_db_session),
) -> OperatorResponse:
	existing = await db.execute(select(Operator).where(Operator.user_id == payload.user_id))
	if existing.scalar_one_or_none() is not None:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="operator already exists")
	op = Operator(user_id=payload.user_id, role=payload.role)
	db.add(op)
	await db.commit()
	await db.refresh(op)
	return OperatorResponse.model_validate(op)


@router.delete("/{operator_id}", status_code=status.HTTP_200_OK)
async def delete_operator(
	operator_id: int,
	_: dict = Depends(require_admin),
	db: AsyncSession = Depends(get_db_session),
) -> None:
	result = await db.execute(select(Operator).where(Operator.id == operator_id))
	op = result.scalar_one_or_none()
	if op is None:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="not found")
	await db.execute(delete(Operator).where(Operator.id == operator_id))
	await db.commit()
	return {"status": "deleted"}



