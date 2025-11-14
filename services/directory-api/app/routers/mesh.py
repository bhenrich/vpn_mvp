from __future__ import annotations

from typing import Sequence, Any, Tuple
import uuid

from fastapi import APIRouter, Depends, HTTPException, status
from sqlalchemy import select
from sqlalchemy.ext.asyncio import AsyncSession

from app.db import get_db_session
from app.models import Node, Region, ClientDevice
from app.schemas import (
	MeshPathRequest,
	MeshPathResponse,
	NodeRef,
	MeshClientConfigRequest,
	MeshClientConfigResponse,
	MeshClientPeer,
)
from app.grpc_server import enqueue_config_for_public_key
from app.security import require_operator, verify_bearer
from app.services.client_devices import ensure_client_ipv4

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


def _pick_nodes(nodes: Sequence[Node], mode: str) -> tuple[Node, Node | None]:
	if not nodes:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="No nodes available in region")

	if mode == "single":
		return nodes[0], None

	if len(nodes) < 2:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="Not enough nodes for multi-hop")
	entry, exit = nodes[0], nodes[1]
	if entry.id == exit.id:
		if len(nodes) >= 3:
			exit = nodes[2]
		else:
			raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="Not enough distinct nodes for multi-hop")
	return entry, exit


def _claims_user_id(claims: dict[str, Any]) -> str:
	sub = str(claims.get("sub") or "").strip()
	if not sub:
		raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="missing subject claim")
	return sub


def _resolve_endpoint(node: Node) -> tuple[str, int]:
	host = node.public_endpoint or (node.egress_ips[0] if node.egress_ips else None)
	if not host:
		raise HTTPException(status_code=status.HTTP_409_CONFLICT, detail="Node missing public endpoint")
	port = node.listen_port or 51820
	return host, port


def _build_client_peer_config(device: ClientDevice, client_ip_v4: str) -> bytes:
	lines: list[str] = ["[Peer]", f"PublicKey = {device.wg_pubkey}", f"AllowedIPs = {client_ip_v4}/32", "PersistentKeepalive = 25", ""]
	return "\n".join(lines).encode("utf-8")


async def _provision_client_peer(node: Node, device: ClientDevice, client_ip_v4: str) -> None:
	wg_conf = _build_client_peer_config(device, client_ip_v4)
	await enqueue_config_for_public_key(node.public_key, wg_conf)


@router.post("/path", response_model=MeshPathResponse)
async def select_path(
	payload: MeshPathRequest,
	_: dict = Depends(require_operator),
	db: AsyncSession = Depends(get_db_session),
) -> MeshPathResponse:
	await _get_region(db, payload.region_id)
	nodes = await _load_region_nodes(db, payload.region_id)
	entry, exit = _pick_nodes(nodes, payload.mode)
	return MeshPathResponse(mode=payload.mode, entry=_to_ref(entry), exit=_to_ref(exit) if exit else None)


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


@router.post("/client-config", response_model=MeshClientConfigResponse)
async def client_config(
	payload: MeshClientConfigRequest,
	claims: dict[str, Any] = Depends(verify_bearer),
	db: AsyncSession = Depends(get_db_session),
) -> MeshClientConfigResponse:
	await _get_region(db, payload.region_id)
	nodes = await _load_region_nodes(db, payload.region_id)
	entry, exit_node = _pick_nodes(nodes, payload.mode)
	user_id = _claims_user_id(claims)

	device = await db.get(ClientDevice, payload.device_id)
	if device is None or device.user_id != user_id:
		raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="device not found")
	if not device.wg_pubkey:
		raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="device missing WireGuard key")

	client_ip_v4 = await ensure_client_ipv4(db, device)
	device.status = "provisioned"
	device.touch()

	allowed_ips_v4 = ["0.0.0.0/0"] if payload.full_tunnel else ["10.66.0.0/24"]
	allowed_ips_v6 = ["::/0"] if payload.full_tunnel and payload.ipv6 else []
	dns_servers = ["10.66.0.1"]

	entry_host, entry_port = _resolve_endpoint(entry)
	entry_peer = MeshClientPeer(
		node_id=entry.id,
		public_key=entry.public_key,
		internal_wg_ip=entry.internal_wg_ip,
		endpoint_host=entry_host,
		endpoint_port=entry_port,
	)

	exit_peer: MeshClientPeer | None = None
	if exit_node is not None:
		exit_host, exit_port = _resolve_endpoint(exit_node)
		exit_peer = MeshClientPeer(
			node_id=exit_node.id,
			public_key=exit_node.public_key,
			internal_wg_ip=exit_node.internal_wg_ip,
			endpoint_host=exit_host,
			endpoint_port=exit_port,
		)

	await _provision_client_peer(entry, device, client_ip_v4)
	await db.commit()
	await db.refresh(device)

	return MeshClientConfigResponse(
		device_id=device.id,
		client_ip_v4=client_ip_v4,
		client_ip_v6=device.client_ip_v6,
		dns_servers=dns_servers,
		allowed_ips_v4=allowed_ips_v4,
		allowed_ips_v6=allowed_ips_v6,
		keepalive_seconds=25,
		entry=entry_peer,
		exit=exit_peer,
	)


