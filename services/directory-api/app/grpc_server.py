from __future__ import annotations

import asyncio
import logging
import os
from pathlib import Path
from typing import AsyncIterator

import grpc
from grpc_tools import protoc  # type: ignore

from app.settings import get_settings
from app.db import get_session_factory
from app.models import Node

_server: grpc.aio.Server | None = None
_log = logging.getLogger("directory.grpc")

# in-memory config stream registries (keyed by node public_key)
_stream_queues: dict[str, "asyncio.Queue[bytes]"] = {}
_revisions: dict[str, int] = {}
_registry_lock = asyncio.Lock()


def _proto_src_dir() -> str:
	# In repo layout, services/common/protos sits two levels up from app/.
	root = Path(__file__).resolve().parents[3]
	return str((root / "services" / "common" / "protos").resolve())


def _gen_out_dir() -> Path:
	return Path(__file__).resolve().parent / "_gen"


def ensure_protos_compiled() -> None:
	out_dir = _gen_out_dir()
	out_dir.mkdir(parents=True, exist_ok=True)
	init_file = out_dir / "__init__.py"
	init_file.touch(exist_ok=True)
	# compile directory.proto
	proto_path = _proto_src_dir()
	result = protoc.main(
		[
			"protoc",
			f"-I{proto_path}",
			f"--python_out={out_dir}",
			f"--grpc_python_out={out_dir}",
			str(Path(proto_path) / "directory.proto"),
		]
	)
	if result != 0:
		raise RuntimeError("protoc failed to generate grpc stubs for directory.proto")

	# ensure generated package is importable
	if str(out_dir) not in os.sys.path:
		os.sys.path.append(str(out_dir))


async def start_grpc_server() -> None:
	global _server
	if _server is not None:
		return

	ensure_protos_compiled()
	from directory_pb2_grpc import DirectoryServiceServicer, add_DirectoryServiceServicer_to_server  # type: ignore
	from directory_pb2 import (  # type: ignore
		NodeRegistrationRequest,
		NodeRegistrationResponse,
		HeartbeatRequest,
		HeartbeatResponse,
		ConfigUpdate,
		NodeIdentity,
	)

	class DirectoryService(DirectoryServiceServicer):  # type: ignore
		async def Register(self, request: NodeRegistrationRequest, context: grpc.aio.ServicerContext) -> NodeRegistrationResponse:  # type: ignore
			# Skeleton: acknowledge but indicate HTTP path for now
			return NodeRegistrationResponse(node_id="", status="use_http_registration")

		async def Heartbeat(self, request: HeartbeatRequest, context: grpc.aio.ServicerContext) -> HeartbeatResponse:  # type: ignore
			# Skeleton: accept heartbeat but no stateful effects yet
			return HeartbeatResponse(node_id="", status=request.status or "ok")

		async def StreamConfig(self, request: NodeIdentity, context: grpc.aio.ServicerContext) -> AsyncIterator[ConfigUpdate]:  # type: ignore
			pub = request.public_key
			_log.info("StreamConfig opened for %s", pub)
			queue: "asyncio.Queue[bytes]"
			async with _registry_lock:
				queue = _stream_queues.get(pub) or asyncio.Queue(maxsize=16)
				_stream_queues[pub] = queue
				_revisions.setdefault(pub, 0)
			# yield updates as they arrive
			while True:
				if context.cancelled():
					break
				try:
					wg_conf = await asyncio.wait_for(queue.get(), timeout=60.0)
				except asyncio.TimeoutError:
					continue
				rev = 0
				async with _registry_lock:
					_revisions[pub] = _revisions.get(pub, 0) + 1
					rev = _revisions[pub]
				# look up node_id for metadata (best-effort)
				node_id_str = ""
				try:
					sf = get_session_factory()
					async with sf() as session:
						result = await session.execute(
							Node.__table__.select().where(Node.public_key == pub)  # type: ignore[attr-defined]
						)
						row = result.first()
						if row is not None:
							node_id_str = str(row[0])
				except Exception as e:  # noqa: BLE001
					_log.warning("node lookup for %s failed: %s", pub, e)
				yield ConfigUpdate(node_id=node_id_str, wg_config=wg_conf, revision=rev)
			_log.info("StreamConfig closed for %s", pub)

	_server = grpc.aio.server()
	add_DirectoryServiceServicer_to_server(DirectoryService(), _server)  # type: ignore
	address = f"0.0.0.0:{get_settings().grpc_port}"

	# TLS / mTLS
	settings = get_settings()
	if settings.grpc_tls_enabled:
		cert_path = settings.grpc_tls_cert_path
		key_path = settings.grpc_tls_key_path
		if not cert_path or not key_path:
			raise RuntimeError("gRPC TLS is enabled but cert/key paths are not configured")
		with open(cert_path, "rb") as f:
			cert_chain = f.read()
		with open(key_path, "rb") as f:
			private_key = f.read()

		client_ca = None
		require_client_auth = False
		if settings.grpc_tls_client_ca_path:
			with open(settings.grpc_tls_client_ca_path, "rb") as f:
				client_ca = f.read()
				require_client_auth = True

		creds = grpc.ssl_server_credentials(
			[(private_key, cert_chain)],
			root_certificates=client_ca,
			require_client_auth=require_client_auth,
		)
		_server.add_secure_port(address, creds)
	else:
		_server.add_insecure_port(address)
	await _server.start()
	_log.info("gRPC server started on %s", address)


async def stop_grpc_server(grace: float = 1.0) -> None:
	global _server
	if _server is None:
		return
	await _server.stop(grace)
	_server = None
	_log.info("gRPC server stopped")


async def enqueue_config_for_public_key(public_key: str, wg_config: bytes) -> bool:
	"""
	Enqueue a config blob for a node identified by its WireGuard public key.
	Returns True if a waiting stream exists and the update was queued, False otherwise.
	"""
	async with _registry_lock:
		queue = _stream_queues.get(public_key)
	if queue is None:
		_log.info("no active stream for %s; dropping config update", public_key)
		return False
	try:
		queue.put_nowait(wg_config)
		return True
	except asyncio.QueueFull:
		_log.warning("stream queue full for %s; dropping newest update", public_key)
		return False

