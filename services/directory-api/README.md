# directory-api (FastAPI skeleton)

Directory/topology service for node registry, regions catalog, and config distribution.

- See `DEVELOPMENT.md` for local setup, environment configuration, and testing.
- See `DEPLOYMENT.md` for container build, configuration, and deployment notes.

## Quick start (dev)

1. Copy environment template:

   ```bash
   cp env.example .env
   ```

2. Build and run via compose from repo root:

   ```bash
   docker compose up --build directory-api
   ```

3. Health check:

   ```bash
   curl http://localhost:8081/healthz
   ```

Environment

- `PORT` (default 8081): HTTP server port.
- `GRPC_PORT` (default 50051): gRPC server port (dev-only, no mTLS yet).
- `DATABASE_URL`: async SQLAlchemy DSN (e.g., `postgresql+asyncpg://postgres:example@postgres:5432/postgres`).
- `REDIS_URL`: Redis URL for ephemeral state and future feeds.

HTTP API

- `GET /healthz`: service health.
- `GET /regions`: list regions.
- `POST /regions`: create region `{ country_code, city, status?, latency_score? }`.
- `PUT /regions/{id}`: update region partial fields.
- `DELETE /regions/{id}`: delete region.
- `GET /nodes`: list nodes.
- `POST /nodes/register`: register/update node metadata.
- `POST /nodes/heartbeat`: update node status and last_seen.
- `POST /devices/register`: register/update a client device (requires bearer token).
- `POST /mesh/client-config`: allocate client IP + session for a region/mode and push the peer config to the selected node.
- `DELETE /mesh/sessions/{session_id}`: terminate a client session and remove the peer on the node.

gRPC API (skeleton)

- Proto: `services/common/protos/directory.proto`
- Service: `DirectoryService`
- Methods:
  - `Register`: placeholder returns `use_http_registration` (HTTP path active).
  - `Heartbeat`: placeholder echoes status.
  - `StreamConfig`: long-lived stream, no updates yet (keeps connection alive).

Development notes

- On startup the service:
  - Ensures DB schema exists (dev-only; production uses Alembic).
  - Initializes Redis and gRPC server.


