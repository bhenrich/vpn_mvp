## admin-api – Deployment Guide

**Service role**: Admin/fleet control for nodes, feature flags, and operator‑level actions.

### Container image

`admin-api` is packaged via `services/admin-api/Dockerfile`.  
The root `docker-compose.yml` defines the `admin-api` service.

#### Build locally

From the repo root:

```bash
docker compose build admin-api
```

Or directly:

```bash
docker build -t admin-api:local ./services/admin-api
```

### Dependencies

The service requires:

- **Postgres** – persistent store for operators, flags, and node metadata.
- **Redis** – cache/ephemeral state (as indicated in service design).
- **auth-api** – for validating operator tokens via JWKS (`AUTH_JWKS_URL`).

In the dev stack, these are provided by other services in `docker-compose.yml`.

### Environment configuration

The compose stack uses:

- `env_file: ./services/admin-api/.env`

Use `env.example` and `app/settings.py` as a reference and configure:

- `DATABASE_URL` / `database_url` – production Postgres DSN.
- `REDIS_URL` / `redis_url` – production Redis DSN.
- `AUTH_JWKS_URL` / `auth_jwks_url` – internal URL for `auth-api` JWKS endpoint.
- `BOOTSTRAP_OPERATOR_USER_ID` / `bootstrap_operator_user_id` – **deployment-time bootstrap only**; typically unset once initial operators are created.

### Running in the dev compose stack

```bash
docker compose up admin-api
```

The service is reachable at `http://localhost:8082` by default.

### Production deployment considerations

- **Orchestration**: Run as a stateless service in your orchestrator (Kubernetes, Nomad, or VM with `systemd`).
- **Security**:
  - Place `admin-api` on a restricted network segment; it should not be internet‑facing.
  - Front it with an authentication/authorization layer for operators (SSO, VPN, mTLS).
  - Enforce mTLS between `admin-api` and other internal services where possible.
- **Secrets**:
  - Store database/Redis credentials and JWKS URLs in a secret manager.
  - Avoid embedding secrets in the container image or committing them to git.
- **Scaling**:
  - Stateless; can be horizontally scaled.
  - Throughput is low compared to data plane; focus on availability and strong access controls.

### Health checks

- Liveness/readiness: `GET /healthz`.

Configure your orchestrator to use this endpoint to gate traffic and implement rolling updates.

### Database migrations

`admin-api` uses async SQLAlchemy and a DB schema initialised by `app.db.init_db()`.  
For production, you should manage schema changes via migrations (e.g. Alembic) in a controlled job before deploying new versions.


