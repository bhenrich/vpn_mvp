## Dev Compose Stack

The root `docker-compose.yml` spins up the entire control plane plus multiple `node-agent`
containers so the desktop client can exercise region selection end-to-end.

### Services

- `auth-api`, `directory-api`, `admin-api` – FastAPI control-plane services.
- `postgres`, `redis` – shared datastore/cache instances.
- `node-agent-de-berlin`, `node-agent-de-munich` – WireGuard nodes that self-register with directory-api.
- `directory-seed` – lightweight helper that ensures the `DE/Berlin` and `DE/Munich` regions exist before the nodes start.
- `prometheus`, `grafana`, `jaeger`, `alertmanager`, `reverse-proxy` – optional observability components.

### Hostname overrides

Both node containers advertise friendly hostnames (`de-berlin.dev.local`, `de-munich.dev.local`).
Add entries to your workstation's hosts file pointing those names at the Docker host IP so the
desktop client can resolve them:

```
192.168.0.183   de-berlin.dev.local
192.168.0.183   de-munich.dev.local
```

Replace `192.168.0.183` with the LAN IP of the machine running Docker.

### Bringing the stack up

```powershell
docker compose up -d postgres redis auth-api directory-api admin-api directory-seed `
  node-agent-de-berlin node-agent-de-munich
```

Each node container exposes its WireGuard port on the host (`51820/udp` and `51821/udp` respectively).
The directory service runs on `http://localhost:8081` while the gRPC config stream listens on port `50051`.

### Tear down

```powershell
docker compose down
```

Add `-v` if you want to remove the Postgres/Grafana volumes as well.
