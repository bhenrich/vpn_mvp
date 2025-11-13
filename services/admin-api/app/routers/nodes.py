from __future__ import annotations

import hashlib
from typing import Any

from fastapi import APIRouter, Depends, status
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.models import AuditEvent
from app.schemas import AcceptedResponse, NodeLifecycleRequest
from app.security import require_admin, require_operator

router = APIRouter(prefix="/nodes", tags=["nodes"])


def _hash_subject(subject: str) -> str:
	return hashlib.sha256(subject.encode("utf-8")).hexdigest()


async def _record_audit(
	db: AsyncSession,
	event_type: str,
	subject: str,
	payload: dict[str, Any],
	retention_tag: str = "short",
) -> None:
	audit = AuditEvent(
		type=event_type,
		subject_hash=_hash_subject(subject),
		payload_redacted=payload,
		retention_tag=retention_tag,
	)
	db.add(audit)
	await db.commit()


@router.post("/{node_id}/enroll", response_model=AcceptedResponse, status_code=status.HTTP_202_ACCEPTED)
async def enroll_node(
	node_id: int,
	payload: NodeLifecycleRequest,
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> AcceptedResponse:
	await _record_audit(db, "node.enroll", f"node:{node_id}", {"reason": payload.reason or ""}, "standard")
	return AcceptedResponse()


@router.post("/{node_id}/drain", response_model=AcceptedResponse, status_code=status.HTTP_202_ACCEPTED)
async def drain_node(
	node_id: int,
	payload: NodeLifecycleRequest,
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> AcceptedResponse:
	await _record_audit(db, "node.drain", f"node:{node_id}", {"reason": payload.reason or ""}, "standard")
	return AcceptedResponse()


@router.post("/{node_id}/upgrade", response_model=AcceptedResponse, status_code=status.HTTP_202_ACCEPTED)
async def upgrade_node(
	node_id: int,
	payload: NodeLifecycleRequest,
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> AcceptedResponse:
	await _record_audit(
		db,
		"node.upgrade",
		f"node:{node_id}",
		{"version": payload.version or "", "reason": payload.reason or ""},
		"standard",
	)
	return AcceptedResponse()


@router.post("/{node_id}/decommission", response_model=AcceptedResponse, status_code=status.HTTP_202_ACCEPTED)
async def decommission_node(
	node_id: int,
	payload: NodeLifecycleRequest,
	_: dict = Depends(require_admin),
	db: AsyncSession = Depends(get_db_session),
) -> AcceptedResponse:
	await _record_audit(db, "node.decommission", f"node:{node_id}", {"reason": payload.reason or ""}, "long")
	return AcceptedResponse()



