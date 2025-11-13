from __future__ import annotations

from typing import Sequence
import uuid

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.models import Node, Region
from app.schemas import MeshPathRequest, MeshPathResponse, NodeRef
from app.grpc_server import enqueue_config_for_public_key
from app.security import require_operator

router = APIRouter()


async def _get_region(db: AsyncSession, region_id: uuid.UUID) -> Region:
	result = await db.get(Region, region_id)
	if result is None:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="Region not found")
	return result


async def _load_region_nodes(db: AsyncSession, region_id: uuid.UUID) -> Sequence[Node]:
	q = (
		select(Node)
		.where(Node.region_id == region_id)
		.where(Node.status.in_(("online", "registered")))
		.order_by(Node.last_seen_at.desc().nullslast(), Node.public_key)
	)
	result = await db.execute(q)
	return list(result.scalars().all())


def _to_ref(n: Node) -> NodeRef:
	return NodeRef(
		id=n.id,
		public_key=n.public_key,
		internal_wg_ip=n.internal_wg_ip,  # type: ignore[arg-type]
		egress_ips=n.egress_ips or [],
	)


@router.post("/path", response_model=MeshPathResponse)
async def select_path(
	payload: MeshPathRequest,
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> MeshPathResponse:
	# Validate region
	await _get_region(db, payload.region_id)
	# Load candidate nodes
	nodes = await _load_region_nodes(db, payload.region_id)
	if not nodes:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="No nodes available in region")

	if payload.mode == "single":
		entry = nodes[0]
		return MeshPathResponse(mode="single", entry=_to_ref(entry), exit=None)

	# multi-hop requires at least two distinct nodes
	if len(nodes) < 2:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="Not enough nodes for multi-hop")
	entry, exit = nodes[0], nodes[1]
	if entry.id == exit.id:
		# Extremely unlikely given ordering, but guard anyway
		if len(nodes) >= 3:
			exit = nodes[2]
		else:
			raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="Not enough distinct nodes for multi-hop")
	return MeshPathResponse(mode="multi", entry=_to_ref(entry), exit=_to_ref(exit))


def _ensure_wg_ip(node: Node, role: str) -> str:
	if not node.internal_wg_ip:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail=f"Node {role} missing internal_wg_ip")
	return node.internal_wg_ip


def _build_peer_only_config(local: Node, peer: Node) -> bytes:
	# Build a minimal peer-only wg config for setconf; interface keys/addresses are managed by the node
	peer_ip = _ensure_wg_ip(peer, "peer")
	lines: list[str] = []
	lines.append("[Peer]")
	lines.append(f"PublicKey = {peer.public_key}")
	lines.append(f"AllowedIPs = {peer_ip}/32")
	lines.append("PersistentKeepalive = 25")
	lines.append("")
	return ("\n".join(lines)).encode("utf-8")


@router.post("/apply", response_model=MeshPathResponse, status_code=status.HTTP_202_ACCEPTED)
async def apply_path(
	payload: MeshPathRequest,
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> MeshPathResponse:
	# Reuse selection logic
	selection = await select_path(payload, db)

	# Fetch full node records
	result = await db.execute(select(Node).where(Node.public_key.in_(
		[selection.entry.public_key] + ([selection.exit.public_key] if selection.exit else [])
	)))
	pub_to_node: dict[str, Node] = {n.public_key: n for n in result.scalars().all()}

	entry_node = pub_to_node.get(selection.entry.public_key)
	if entry_node is None:
		raise HTTPException(status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, detail="Entry node not found for apply")

	if selection.mode == "single":
		# No inter-node peer config needed; in future we might clear peers
		return selection

	if not selection.exit:
		raise HTTPException(status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, detail="Exit node missing for multi-hop")
	exit_node = pub_to_node.get(selection.exit.public_key)
	if exit_node is None:
		raise HTTPException(status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, detail="Exit node not found for apply")

	# Build and enqueue peer configs in both directions
	entry_conf = _build_peer_only_config(entry_node, exit_node)
	exit_conf = _build_peer_only_config(exit_node, entry_node)

	await enqueue_config_for_public_key(entry_node.public_key, entry_conf)
	await enqueue_config_for_public_key(exit_node.public_key, exit_conf)

	return selection


