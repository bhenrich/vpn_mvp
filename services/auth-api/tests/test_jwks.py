from fastapi.testclient import TestClient

from app.main import app


def test_jwks() -> None:
	client = TestClient(app)
	resp = client.get("/.well-known/jwks.json")
	assert resp.status_code == 200
	data = resp.json()
	assert "keys" in data
	assert isinstance(data["keys"], list)
	assert len(data["keys"]) >= 1
	first = data["keys"][0]
	assert first.get("kty") == "RSA"
	assert "kid" in first and "n" in first and "e" in first


