## auth-api – Deployment Guide

**Service role**: Authentication and user management, token issuance (JWT/JWKS), device registration, and GDPR workflows.

### Container image

The service is containerised via `services/auth-api/Dockerfile`.  
The root `docker-compose.yml` builds and runs this image under the `auth-api` service.

#### Building locally

From the repo root:

```bash
docker compose build auth-api
```

Or directly:

```bash
docker build -t auth-api:local ./services/auth-api
```

### Dependencies

At minimum, `auth-api` requires:

- **Postgres** – for persistent data (`users`, `devices`, `sessions`, policies, etc.).
- **Redis** – for short‑lived state, counters, and token-related state (see `PLAN.md`).

In the dev stack, these are provided by the `postgres` and `redis` services defined in the root `docker-compose.yml`.

### Environment configuration (container)

The compose file references an env file:

- `env_file: ./services/auth-api/.env`

You should derive production values from `env.example` and the defaults in `app/config.py`:

- `DATABASE_URL` – production Postgres DSN.
- `REDIS_URL` – production Redis DSN.
- `JWT_ISSUER` and TTLs – tuned to your security requirements.
- Argon2 parameters – may be increased in production if tolerated by latency budgets.

### Running in the dev compose stack

From the repo root:

```bash
docker compose up auth-api
```

This will also start its dependencies (Postgres, Redis).  
The service will listen on `0.0.0.0:8080` by default and be bound to `localhost:8080` on the host.

### Production deployment considerations

While this repo ships a dev‑oriented compose stack, production deployments should consider:

- **Orchestration**: Kubernetes, Nomad, or VM-based deployment with `systemd`.
- **TLS termination**: front `auth-api` with a reverse proxy or ingress controller (nginx, Envoy, etc.) terminating TLS.
- **mTLS and service-to-service auth**: other services (e.g. `directory-api`, `admin-api`) validate JWTs using JWKS (`/.well-known/jwks.json`) over mTLS‑secured channels.
- **Secrets management**: inject database credentials, Redis URIs, and JWT signing keys via a secret manager (Vault, AWS/GCP/Azure secrets) rather than `.env` files.
- **Scaling**: the service is stateless; scale horizontally behind a load balancer as needed.

### Health checks and readiness

- **Liveness**: `GET /healthz` returns a simple status payload.
- **Readiness**: reuse `/healthz` or add a dedicated readiness endpoint that checks database/Redis connectivity before admitting traffic.

Wiring these into your orchestrator’s health checking will allow safe rolling updates.

### Migrations in deployment

Database migrations are managed via Alembic. Typical production pattern:

1. Build and push the `auth-api` image.
2. Run a one‑off task (job) that executes `alembic upgrade head`.
3. Deploy the new application version.

Avoid running migrations automatically on every container boot in production unless you have strict safeguards.


