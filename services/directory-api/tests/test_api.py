import os
import sys
from pathlib import Path
import uuid

from fastapi.testclient import TestClient

# Ensure 'app' package root is importable
sys.path.insert(0, str((Path(__file__).resolve().parents[1]).resolve()))
# Ensure generated stubs (which use absolute imports) are importable
sys.path.insert(0, str((Path(__file__).resolve().parents[1] / "app" / "_gen").resolve()))

# Avoid external services during tests
os.environ.setdefault("DATABASE_URL", "sqlite+aiosqlite:///./.pytest-directory-api.db")
os.environ.setdefault("REDIS_URL", "redis://localhost:6379/0")

from app.main import app  # noqa: E402
from app.security import verify_bearer, require_operator, require_admin  # noqa: E402

# Disable app startup/shutdown hooks that touch external services for unit tests
app.router.on_startup.clear()
app.router.on_shutdown.clear()

_TEST_USER_ID = uuid.uuid4()


def _fake_claims() -> dict[str, str]:
	return {"sub": str(_TEST_USER_ID), "roles": ["operator", "admin"]}


app.dependency_overrides[verify_bearer] = lambda: _fake_claims()
app.dependency_overrides[require_operator] = lambda: _fake_claims()
app.dependency_overrides[require_admin] = lambda: _fake_claims()

_db_initialized = False


def _ensure_db() -> None:
	# Initialize DB schema for tests (SQLite) from sync context, once
	global _db_initialized
	if _db_initialized:
		return
	import asyncio
	from app.db import init_db
	asyncio.run(init_db())
	_db_initialized = True


async def _ensure_db_async() -> None:
	global _db_initialized
	if _db_initialized:
		return
	from app.db import init_db
	await init_db()
	_db_initialized = True


async def _dispose_db_async() -> None:
	# Dispose async engine to clean up aiosqlite threads
	from app.db import get_engine
	engine = get_engine()
	await engine.dispose()

def test_healthz() -> None:
	with TestClient(app) as client:
		resp = client.get("/healthz")
		assert resp.status_code == 200
		assert resp.json() == {"status": "ok"}


def test_regions_crud() -> None:
	random_city = f"TestCity-{uuid.uuid4().hex[:8]}"
	_ensure_db()
	with TestClient(app) as client:
		# list initially (may be non-empty if reused DB); ensure request works
		resp = client.get("/regions")
		assert resp.status_code == 200

		# create region
		payload = {"country_code": "US", "city": random_city, "status": "active", "latency_score": 10.5}
		resp = client.post("/regions", json=payload)
		assert resp.status_code == 201, resp.text
		region = resp.json()
		assert region["country_code"] == "US"
		assert region["city"] == random_city
		assert region["status"] == "active"
		assert region["latency_score"] == 10.5
		region_id = region["id"]

		# list should include the new region
		resp = client.get("/regions")
		assert resp.status_code == 200
		assert any(r["id"] == region_id for r in resp.json())

		# update region
		update = {"status": "disabled", "latency_score": 5.0}
		resp = client.put(f"/regions/{region_id}", json=update)
		assert resp.status_code == 200
		updated = resp.json()
		assert updated["status"] == "disabled"
		assert updated["latency_score"] == 5.0

		# delete region
		resp = client.delete(f"/regions/{region_id}")
		assert resp.status_code == 204


def test_node_register_and_heartbeat() -> None:
	random_city = f"NodeCity-{uuid.uuid4().hex[:8]}"
	public_key = f"pk-{uuid.uuid4()}"
	_ensure_db()
	with TestClient(app) as client:
		# create a region to attach node
		resp = client.post("/regions", json={"country_code": "DE", "city": random_city})
		assert resp.status_code == 201
		region_id = resp.json()["id"]

		# register node
		reg_payload = {
			"public_key": public_key,
			"region_id": region_id,
			"agent_version": "0.0.1",
			"wg_rs_version": "0.3.0",
			"internal_wg_ip": "10.0.0.2",
			"egress_ips": ["203.0.113.10"],
			"capabilities": {"egress": "true"},
		}
		resp = client.post("/nodes/register", json=reg_payload)
		assert resp.status_code == 201, resp.text
		node = resp.json()
		assert node["public_key"] == public_key
		assert node["status"] == "registered"
		assert node["region_id"] == region_id

		# heartbeat -> status online
		hb_payload = {"public_key": public_key, "status": "online", "agent_version": "0.0.2"}
		resp = client.post("/nodes/heartbeat", json=hb_payload)
		assert resp.status_code == 200
		node = resp.json()
		assert node["status"] == "online"
		assert node["agent_version"] == "0.0.2"


def test_mesh_apply_streams_config() -> None:
	import asyncio
	from app.settings import get_settings
	from app._gen import directory_pb2_grpc as d_grpc  # type: ignore
	from app._gen import directory_pb2 as d_pb2  # type: ignore
	import grpc  # type: ignore
	from app.grpc_server import start_grpc_server, stop_grpc_server

	random_city = f"MeshCity-{uuid.uuid4().hex[:8]}"
	pk1 = f"pk-{uuid.uuid4()}"
	pk2 = f"pk-{uuid.uuid4()}"

	async def run(grpc_addr: str, region_id: str) -> None:
		await start_grpc_server()
		async with grpc.aio.insecure_channel(grpc_addr) as channel:
			stub = d_grpc.DirectoryServiceStub(channel)
			# open streams for both nodes
			stream1 = stub.StreamConfig(d_pb2.NodeIdentity(public_key=pk1, agent_version="t", wg_rs_version="t"))
			stream2 = stub.StreamConfig(d_pb2.NodeIdentity(public_key=pk2, agent_version="t", wg_rs_version="t"))
			# give server a moment to register streams
			await asyncio.sleep(0.2)

			# apply multi-hop path to push configs
			await _ensure_db_async()
			with TestClient(app) as client:
				# register nodes
				for pk, ip in ((pk1, "10.10.0.2"), (pk2, "10.10.0.3")):
					resp = client.post("/nodes/register", json={
						"public_key": pk, "region_id": region_id, "internal_wg_ip": ip, "agent_version": "x", "wg_rs_version": "y"
					})
					assert resp.status_code == 201, resp.text
					resp = client.post("/nodes/heartbeat", json={"public_key": pk, "status": "online"})
					assert resp.status_code == 200
				# apply multi-hop
				resp = client.post("/mesh/apply", json={"region_id": region_id, "mode": "multi"})
				assert resp.status_code == 202, resp.text

			# Expect one config update on each stream
			async def recv_one(stream):
				msg = await asyncio.wait_for(stream.read(), timeout=5.0)
				assert msg is not None
				assert msg.revision >= 1
				assert b"[Peer]" in msg.wg_config

			await asyncio.gather(recv_one(stream1), recv_one(stream2))
		await _dispose_db_async()
		await stop_grpc_server()

	# launch
	grpc_addr = f"localhost:{get_settings().grpc_port}"
	# create a region id deterministically within run() to assert equality
	_ensure_db()
	with TestClient(app) as client:
		resp = client.post("/regions", json={"country_code": "NL", "city": random_city})
		assert resp.status_code == 201
		region_id = resp.json()["id"]

	asyncio.run(run(grpc_addr, region_id))


def test_mesh_config_latency_budget() -> None:
	"""
	Measure end-to-end latency from /mesh/apply to first ConfigUpdate on StreamConfig.
	Target budget: < 1.0s in test environment.
	"""
	import asyncio, time
	from app.settings import get_settings
	from app._gen import directory_pb2_grpc as d_grpc  # type: ignore
	from app._gen import directory_pb2 as d_pb2  # type: ignore
	import grpc  # type: ignore
	from app.grpc_server import start_grpc_server, stop_grpc_server

	random_city = f"LatencyCity-{uuid.uuid4().hex[:8]}"
	pk1 = f"pk-{uuid.uuid4()}"
	pk2 = f"pk-{uuid.uuid4()}"

	async def run(grpc_addr: str, region_id: str) -> None:
		await start_grpc_server()
		async with grpc.aio.insecure_channel(grpc_addr) as channel:
			stub = d_grpc.DirectoryServiceStub(channel)
			stream1 = stub.StreamConfig(d_pb2.NodeIdentity(public_key=pk1, agent_version="t", wg_rs_version="t"))
			stream2 = stub.StreamConfig(d_pb2.NodeIdentity(public_key=pk2, agent_version="t", wg_rs_version="t"))
			# give server a moment to register streams
			await asyncio.sleep(0.2)

			await _ensure_db_async()
			with TestClient(app) as client:
				# register nodes online
				for pk, ip in ((pk1, "10.20.0.2"), (pk2, "10.20.0.3")):
					resp = client.post("/nodes/register", json={
						"public_key": pk, "region_id": region_id, "internal_wg_ip": ip, "agent_version": "x", "wg_rs_version": "y"
					})
					assert resp.status_code == 201
					resp = client.post("/nodes/heartbeat", json={"public_key": pk, "status": "online"})
					assert resp.status_code == 200

				# trigger apply and measure until first update is received on each stream
				start = time.perf_counter()
				resp = client.post("/mesh/apply", json={"region_id": region_id, "mode": "multi"})
				assert resp.status_code == 202, resp.text

			async def recv_one(stream):
				msg = await asyncio.wait_for(stream.read(), timeout=5.0)
				assert msg is not None
				return msg

			msg1, msg2 = await asyncio.gather(recv_one(stream1), recv_one(stream2))
			latency = time.perf_counter() - start
			assert b"[Peer]" in msg1.wg_config and b"[Peer]" in msg2.wg_config
			assert latency < 1.0, f"Config distribution exceeded latency budget: {latency:.3f}s"
		await _dispose_db_async()
		await stop_grpc_server()

	grpc_addr = f"localhost:{get_settings().grpc_port}"
	_ensure_db()
	with TestClient(app) as client:
		resp = client.post("/regions", json={"country_code": "SE", "city": random_city})
		assert resp.status_code == 201
		region_id = resp.json()["id"]

	asyncio.run(run(grpc_addr, region_id))


def test_device_register_and_client_config_flow() -> None:
	_ensure_db()
	with TestClient(app) as client:
		region_resp = client.post("/regions", json={"country_code": "US", "city": f"DeviceCity-{uuid.uuid4().hex[:6]}"})
		assert region_resp.status_code == 201, region_resp.text
		region_id = region_resp.json()["id"]

		node_pk = f"pk-{uuid.uuid4()}"
		node_payload = {
			"public_key": node_pk,
			"region_id": region_id,
			"internal_wg_ip": "10.66.0.1",
			"egress_ips": ["198.51.100.10"],
			"public_endpoint": "198.51.100.10",
			"listen_port": 51820,
		}
		resp = client.post("/nodes/register", json=node_payload)
		assert resp.status_code == 201, resp.text

		device_payload = {
			"device_name": "Arch Laptop",
			"platform": "linux",
			"wg_public_key": f"client-{uuid.uuid4().hex}",
		}
		resp = client.post("/devices/register", json=device_payload)
		assert resp.status_code == 201, resp.text
		device = resp.json()
		assert device["device_name"] == "Arch Laptop"
		assert device["client_ip_v4"].startswith("10.66.0.")
		device_id = device["id"]

		config_payload = {
			"region_id": region_id,
			"mode": "single",
			"device_id": device_id,
			"full_tunnel": True,
			"ipv6": False,
		}
		resp = client.post("/mesh/client-config", json=config_payload)
		assert resp.status_code == 200, resp.text
		cfg = resp.json()
		assert "session_id" in cfg
		session_id = cfg["session_id"]
		assert cfg["device_id"] == device_id
		assert cfg["client_ip_v4"].startswith("10.66.0.")
		assert cfg["allowed_ips_v4"] == ["0.0.0.0/0"]
		assert cfg["entry"]["public_key"] == node_pk
		assert cfg["entry"]["endpoint_host"] == "198.51.100.10"

	# Disconnect session
	resp = client.delete(f"/mesh/sessions/{session_id}")
	assert resp.status_code == 200, resp.text
	session = resp.json()
	assert session["status"] == "ended"
	assert session["id"] == session_id

