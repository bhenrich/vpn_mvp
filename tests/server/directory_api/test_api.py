import sys
import uuid
from pathlib import Path

# Ensure the directory-api app package is importable when running from repo root
root = Path(__file__).resolve().parents[3]
service_app_dir = root / "services" / "directory-api"
if str(service_app_dir) not in sys.path:
	sys.path.insert(0, str(service_app_dir))

from fastapi.testclient import TestClient
from app.main import app


def test_healthz() -> None:
	with TestClient(app) as client:
		resp = client.get("/healthz")
		assert resp.status_code == 200
		assert resp.json() == {"status": "ok"}


def test_regions_crud() -> None:
	random_city = f"TestCity-{uuid.uuid4().hex[:8]}"
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


