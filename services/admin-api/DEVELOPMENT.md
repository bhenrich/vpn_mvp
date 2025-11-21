## admin-api – Development Guide

**Service role**: Admin/fleet control for nodes and feature flags (WS4).

### Prerequisites

- **Runtime**: Python 3.11+.
- **Dependencies**: See `requirements.txt`.
- **Database**: Postgres 16 (or compatible).
- **Cache**: Redis 7.
- **Tooling**:
  - `docker` and `docker compose` (recommended for the dev stack).

### Environment configuration

From `services/admin-api`:

```bash
cp env.example .env
```

Key variables (see `app/settings.py`):

- `PORT` / `port` – HTTP listen port (default `8082`).
- `DATABASE_URL` / `database_url` – async SQLAlchemy DSN, e.g. `postgresql+asyncpg://postgres:example@postgres:5432/postgres`.
- `REDIS_URL` / `redis_url` – Redis DSN, e.g. `redis://redis:6379/0`.
- `AUTH_JWKS_URL` / `auth_jwks_url` – JWKS endpoint for token validation.
- `BOOTSTRAP_OPERATOR_USER_ID` / `bootstrap_operator_user_id` – optional; when set, startup will ensure an `Operator` row for the given user ID.

### Running in the full dev stack

From the repo root:

```bash
docker compose up --build admin-api
```

This will:

- Build `admin-api` from `services/admin-api/Dockerfile`.
- Start dependencies (Postgres, Redis, other APIs) as defined in `docker-compose.yml`.
- Expose the API on `http://localhost:8082`.

Health check:

```bash
curl http://localhost:8082/healthz
```

### Running directly with uvicorn

Create a virtualenv and install dependencies:

```bash
cd services/admin-api
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -r requirements.txt
cp env.example .env
```

Ensure Postgres and Redis are running (you can reuse the dev compose stack for those).

Run the service:

```bash
uvicorn app.main:app --host 0.0.0.0 --port 8082
```

On startup, the service will:

- Initialize the database schema via `app.db.init_db()`.
- Optionally bootstrap an operator user, if `bootstrap_operator_user_id` is set.

### API surface (high level)

Routers live in `app/routers`:

- `operators` – operator management.
- `flags` – feature flag management.
- `nodes` – node and fleet‑related admin operations.

FastAPI will expose OpenAPI docs at `/docs` in dev.

### Running tests

From `services/admin-api`:

```bash
pytest
```

Tests currently focus on import/health smoke checks; extend as you add functionality.

### Observability

`admin-api` uses `services.common.observability.init_observability` for metrics/traces.  
Hook this into Prometheus/Grafana via the monitoring stack in `monitoring/` and `ops/` as needed.


