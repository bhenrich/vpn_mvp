## auth-api – Development Guide

**Service role**: Authentication and user management (users, devices, sessions, JWT/JWKS, GDPR endpoints).  
For full architecture context, see `PLAN.md` in the repo root.

### Prerequisites

- **Runtime**: Python 3.11+ (recommended to use `pyenv` or similar).
- **Dependencies**: See `requirements.txt`.
- **Database**: Postgres 16 (or compatible).
- **Cache**: Redis 7.
- **Tooling**:
  - `docker` and `docker compose` (for running the full dev stack).
  - `make` (optional, for root-level helpers if/when added).

### Environment configuration

Use the provided template and adjust as needed:

```bash
cp env.example .env
```

Key environment variables (see `app/config.py` for defaults):

- `PORT` – HTTP listen port (default `8080`).
- `DATABASE_URL` – SQLAlchemy DSN, e.g. `postgresql+psycopg://postgres:example@postgres:5432/postgres`.
- `REDIS_URL` – Redis DSN, e.g. `redis://redis:6379/0`.
- `JWT_ISSUER`, `JWT_ACCESS_TTL_MINUTES`, `JWT_REFRESH_TTL_DAYS` – token configuration.
- Argon2 tuning knobs: `ARGON2_TIME_COST`, `ARGON2_MEMORY_COST`, `ARGON2_PARALLELISM`.

### Running in the full dev stack (recommended)

From the repo root, with Docker running:

```bash
docker compose up --build auth-api
```

This will:

- Build the `auth-api` image from `services/auth-api/Dockerfile`.
- Start Postgres and Redis defined in the root `docker-compose.yml`.
- Expose the API on `http://localhost:8080`.

Health check:

```bash
curl http://localhost:8080/healthz
```

### Running the service directly (local Python)

Create and activate a virtualenv:

```bash
cd services/auth-api
python -m venv .venv
source .venv/bin/activate  # Windows: .venv\Scripts\activate
pip install -r requirements.txt
cp env.example .env
```

Make sure Postgres and Redis are running and reachable from your machine (you can use the compose services for this).

Run with `uvicorn`:

```bash
uvicorn app.main:app --host 0.0.0.0 --port 8080
```

The service will also bootstrap seed users on startup via `app/bootstrap.py`.

### Useful dev endpoints

- Health: `GET /healthz`
- JWKS: `GET /.well-known/jwks.json`
- Auth flows and device login examples are documented in `README.md`.

### Database migrations

Alembic configuration lives in `alembic.ini` and `migrations/`.

Example commands (from `services/auth-api`):

```bash
alembic upgrade head    # apply migrations
alembic revision --autogenerate -m "description"  # create new migration
```

In development, you’ll typically:

1. Update models in `app/models.py`.
2. Autogenerate a migration.
3. Apply it to your local database.

### Running tests

From `services/auth-api`:

```bash
pytest
```

The tests include:

- Import/health smoke tests.
- JWKS and token issuance behaviour.

### Observability in dev

`auth-api` uses `services.common.observability.init_observability` for metrics and traces.  
In the dev stack, Prometheus/Grafana integration is wired via `monitoring/` and `ops/`; see those directories and `docs/` for more details.

### Coding standards

- Python formatting / linting is handled via the repo’s CI (see `PLAN.md` and any configured tools like `ruff`, `mypy`).
- Keep new endpoints documented via FastAPI docstrings and OpenAPI schemas.


