# admin-api (FastAPI skeleton)

Admin/fleet control service (WS4).

- See `DEVELOPMENT.md` for local setup, environment configuration, and testing.
- See `DEPLOYMENT.md` for container build, configuration, and deployment patterns.

## Quick start (dev)

1. Copy environment template:

   ```bash
   cp env.example .env
   ```

2. Build and run via compose from repo root:

   ```bash
   docker compose up --build admin-api
   ```

3. Health check:

   ```bash
   curl http://localhost:8082/healthz
   ```


