# admin-api (FastAPI skeleton)

Minimal scaffold for admin/fleet control service.

Quick start:

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


