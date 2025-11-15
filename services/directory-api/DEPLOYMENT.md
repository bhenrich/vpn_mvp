## directory-api – Deployment Guide

**Service role**: Directory/topology and config distribution for nodes and devices.

### Container image

The service is packaged by `services/directory-api/Dockerfile`.  
The root `docker-compose.yml` defines the `directory-api` service.

#### Build locally

From the repo root:

```bash
docker compose build directory-api
```

Or directly:

```bash
docker build -t directory-api:local ./services/directory-api
```

### Dependencies

`directory-api` depends on:

- **Postgres** – region, node, mesh, and device‑mapping data.
- **Redis** – ephemeral state, caching, and counters.
- **auth-api** – token verification keys via `auth_jwks_url`.

In dev, these are wired together via the root `docker-compose.yml`.

### Environment configuration

The compose stack uses:

- `env_file: ./services/directory-api/.env`

Production deployments should configure:

- `database_url` – production Postgres DSN.
- `redis_url` – production Redis DSN.
- `auth_jwks_url` – internal JWKS URL for `auth-api`.
- `port` – HTTP port, typically behind an internal load balancer.
- `grpc_port` – gRPC port for node agents.

### Running in the dev compose stack

```bash
docker compose up directory-api
```

The HTTP API is exposed at `http://localhost:8081`, gRPC on `50051` (dev‑only, no mTLS yet).

### Production deployment considerations

- **Network placement**:
  - `directory-api` is control‑plane only; do not expose it directly to the public internet.
  - Node agents and other control‑plane services should talk to it over private networks with mTLS.
- **TLS/mTLS**:
  - Terminate HTTPS for the HTTP API at a reverse proxy/ingress (nginx, Envoy, etc.).
  - Enable and configure TLS/mTLS for the gRPC server as `grpc_tls_*` settings are wired up.
- **Scaling**:
  - Stateless application layer; scale horizontally.
  - Backing Postgres/Redis must be sized and replicated appropriately.
- **Secrets**:
  - Store DB credentials and Redis URIs in a secret manager; **never** bake them into the image.

### Health checks

- Liveness/readiness endpoint: `GET /healthz`.

Use this in your orchestrator for rollout and restart policies.

### Schema management

In dev, `init_db()` creates/updates the schema automatically.  
For production, prefer a dedicated migration pipeline (Alembic or similar) run as a job before new versions are deployed.


