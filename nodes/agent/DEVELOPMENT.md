## node-agent – Development Guide

**Service role**: Rust node agent responsible for establishing WireGuard tunnels, talking to `directory-api` over gRPC, and applying peer configs.

For architecture context, see the Node Agent workstream (WS5) in `PLAN.md`.

### Prerequisites

- **Runtime**: Rust (stable toolchain via `rustup`).
- **Dependencies**:
  - `wireguard` tooling on the host (for now the implementation shells out to system tools).
  - Linux with TUN support for full end‑to‑end behaviour.
- **Docker**: for running in the dev compose stack with correct capabilities.

### Building locally

From the repo root:

```bash
cargo build -p node-agent
```

This compiles the `nodes/agent` crate and any dependencies.

### Running in the dev compose stack (recommended)

From the repo root:

```bash
docker compose up --build node-agent
```

The `node-agent` service is defined in the root `docker-compose.yml` and:

- Builds from `nodes/agent/Dockerfile`.
- Runs with `NET_ADMIN` and `SYS_MODULE` capabilities.
- Mounts `tmpfs` for `/etc/wireguard` and `/var/run/vpn` to keep keys in RAM.
- Connects to `directory-api` (control‑plane) using the configured gRPC address.

### Running the binary directly

You can run the agent on a dev machine (Linux recommended) after building:

```bash
cargo run -p node-agent
```

Key environment variables (see `src/main.rs`):

- `DIRECTORY_ADDR` – directory gRPC endpoint, default `http://127.0.0.1:8081`.
- `MTLS_ENABLED` – set to `1` to enable mTLS to the directory.
  - `MTLS_CA_CERT_PATH` – path to CA certificate (PEM).
  - `MTLS_CLIENT_CERT_PATH` – path to client certificate (PEM).
  - `MTLS_CLIENT_KEY_PATH` – path to client private key (PEM).
- `WG_SERVER_KEY_PATH` – where to store/read the node’s WireGuard private key (default `/var/run/vpn/server.key`).
- `WG_INTERFACE` – WireGuard interface name (default `wg0`).
- `WG_ADDRESS` – WireGuard interface CIDR (default `10.66.0.1/24`).
- `WG_LISTEN_PORT` – WireGuard listen port (default `51820`).
- `ENABLE_ADMISSION` – set to `1` to enable the admission sidecar server.

You’ll need:

- A running `directory-api` accessible via `DIRECTORY_ADDR`.
- Kernel WireGuard support and the ability to create interfaces.

### Testing and bench harness

There is a bench harness under `src/bin/bench.rs` which you can run with:

```bash
cargo run -p node-agent --bin bench
```

Add unit and integration tests under `nodes/agent/src` as functionality grows.


