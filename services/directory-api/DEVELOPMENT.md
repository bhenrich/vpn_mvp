## directory-api – Development Guide

**Service role**: Directory/topology service for node registry, regions catalog, device config distribution, and mesh metadata (WS3).

### Prerequisites

- **Runtime**: Python 3.11+.
- **Dependencies**: See `requirements.txt`.
- **Database**: Postgres 16 (or compatible).
- **Cache**: Redis 7.
- **Tooling**:
  - `docker` and `docker compose` for the full dev stack.

### Environment configuration

From `services/directory-api`:

```bash
cp env.example .env
```

Key variables (see `app/settings.py`):

- `port` – HTTP listen port (default `8081`).
- `grpc_port` – gRPC listen port (default `50051`) for node control‑plane communication.
- `database_url` – async SQLAlchemy DSN, e.g. `postgresql+asyncpg://postgres:example@postgres:5432/postgres`.
- `redis_url` – Redis DSN, e.g. `redis://redis:6379/0`.
- `auth_jwks_url` – JWKS endpoint of `auth-api` used to validate client and node tokens.

### Running in the dev compose stack

From the repo root:

```bash
docker compose up --build directory-api
```

This will:

- Build `directory-api` from `services/directory-api/Dockerfile`.
- Start Postgres, Redis, and `auth-api` as dependencies.
- Expose the HTTP API on `http://localhost:8081`.

Health check:

```bash
curl http://localhost:8081/healthz
```

### Running directly with uvicorn

Create a virtualenv and install dependencies:

```bash
cd services/directory-api
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -r requirements.txt
cp env.example .env
```

Ensure Postgres and Redis are running (you can reuse the dev compose stack for those).

Start the service:

```bash
uvicorn app.main:app --host 0.0.0.0 --port 8081
```

On startup, the service will:

- Initialise the database schema (dev‑only) via `app.db.init_db()`.
- Initialise Redis via `app.redis_client.init_redis()`.
- Start the gRPC server via `app.grpc_server.start_grpc_server()`.

### HTTP API overview

Routers in `app/routers`:

- `regions` (`/regions`) – region catalogue management.
- `nodes` (`/nodes`) – node registry and heartbeats.
- `mesh` (`/mesh`) – mesh‑related configuration.
- `devices` (`/devices`) – device‑oriented directory queries.

FastAPI exposes interactive docs at `/docs` in dev.

### Running tests

From `services/directory-api`:

```bash
pytest
```

Tests cover import/health and core HTTP API behaviour; extend as functionality grows.

### Observability

`directory-api` initialises observability via `services.common.observability.init_observability`.  
Integrate with the Prometheus/Grafana stack under `monitoring/` and `ops/` when running a full environment.


