# node-agent

Rust-based WireGuard data-plane process that bootstraps itself against `directory-api`,
registers its public key/endpoint, and streams peer updates via the gRPC config channel.

## Build

```bash
cargo build -p node-agent
```

## Required environment

| Variable | Purpose |
| --- | --- |
| `DIRECTORY_HTTP_ADDR` | Base URL for the FastAPI control-plane (default `http://127.0.0.1:8081`). |
| `DIRECTORY_GRPC_ADDR` | gRPC address for config streaming (default `http://127.0.0.1:50051`). |
| `PUBLIC_ENDPOINT` | Hostname or IP:port clients should dial (e.g. `de-berlin.dev.local`). |
| `NODE_REGION_ID` | Optional region UUID. If unset, `NODE_REGION_CODE` + `NODE_REGION_CITY` must be provided so the agent can resolve the region. |
| `NODE_REGION_CODE` / `NODE_REGION_CITY` | Country code (ISO-3166-1 alpha-2) and city string used to look up the region when `NODE_REGION_ID` is not supplied. |
| `NODE_EGRESS_IPS` | Optional comma-separated list of egress IPs advertised to the control-plane. |
| `WG_ADDRESS` | WireGuard interface address/prefix (default `10.66.0.1/24`). |
| `WG_LISTEN_PORT` | UDP port to listen on (default `51820`). |
| `WG_INTERFACE` | Interface name (default `wg0`). |

Optional TLS/mTLS config mirrors the previous implementation:
`MTLS_ENABLED`, `MTLS_CA_CERT_PATH`, `MTLS_CLIENT_CERT_PATH`, `MTLS_CLIENT_KEY_PATH`.

## Running in Docker

### Development

From the repo root:

```bash
docker compose up --build node-agent-de-berlin
```

This will:

1. Build the agent image.
2. Bring up a WireGuard interface inside the container.
3. Register the node with directory-api using the configured region + endpoint.
4. Stream peer updates over gRPC.

### Production Deployment

For production deployment on Linux servers, see [DEPLOYMENT.md](./DEPLOYMENT.md) for:
- Complete environment variable reference
- Docker run and docker-compose examples
- Systemd service configuration
- Security best practices
- Health checks and monitoring
- Troubleshooting guide
