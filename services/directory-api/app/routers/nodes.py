from __future__ import annotations

from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.models import Node, Region
from app.schemas import NodeRegisterRequest, NodeHeartbeatRequest, NodeRead

router = APIRouter()


@router.get("/", response_model=list[NodeRead])
async def list_nodes(db: AsyncSession = Depends(get_db_session)) -> list[Node]:
	result = await db.execute(select(Node).order_by(Node.last_seen_at.desc().nullslast(), Node.public_key))
	return list(result.scalars().all())


@router.post("/register", response_model=NodeRead, status_code=status.HTTP_201_CREATED)
async def register_node(payload: NodeRegisterRequest, db: AsyncSession = Depends(get_db_session)) -> Node:
	# Validate region if provided
	if payload.region_id is not None:
		region = await db.get(Region, payload.region_id)
		if region is None:
			raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="Invalid region_id")

	existing = await db.execute(select(Node).where(Node.public_key == payload.public_key))
	node = existing.scalar_one_or_none()
	if node is None:
		node = Node(public_key=payload.public_key)

	node.region_id = payload.region_id
	node.internal_wg_ip = str(payload.internal_wg_ip) if payload.internal_wg_ip else None
	node.egress_ips = [str(ip) for ip in (payload.egress_ips or [])] or None
	node.capabilities = payload.capabilities
	node.agent_version = payload.agent_version
	node.wg_rs_version = payload.wg_rs_version
	node.status = "registered"
	node.last_seen_at = datetime.now(timezone.utc)

	db.add(node)
	await db.commit()
	await db.refresh(node)
	return node


@router.post("/heartbeat", response_model=NodeRead)
async def heartbeat(payload: NodeHeartbeatRequest, db: AsyncSession = Depends(get_db_session)) -> Node:
	result = await db.execute(select(Node).where(Node.public_key == payload.public_key))
	node = result.scalar_one_or_none()
	if node is None:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Node not found")

	if payload.status:
		node.status = payload.status
	if payload.agent_version:
		node.agent_version = payload.agent_version
	if payload.wg_rs_version:
		node.wg_rs_version = payload.wg_rs_version
	node.last_seen_at = datetime.now(timezone.utc)

	await db.commit()
	await db.refresh(node)
	return node


