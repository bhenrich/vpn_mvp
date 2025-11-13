Legacy Stack (Deprecated)
=========================

The previous prototype used:

- Go `auth` service under `server/auth`
- OpenVPN server under `server/openvpn`
- Monitoring stack in `monitoring/`

This stack is deprecated in favor of the new architecture defined in `PLAN.md`:

- Control plane: Python (FastAPI) services (`services/auth-api`, `services/directory-api`, `services/admin-api`)
- Data plane: Rust node agent using `wireguard-rs` (`nodes/agent`)

Migration notes:
----------------
- Root `docker-compose.yml` now targets the new services; the Go/OpenVPN services are no longer referenced.
- The `server/` directory will be moved under `archive/` in a follow-up change to preserve history while removing it from active dev paths.
- Existing monitoring under `monitoring/` remains for reference; future configs will live in `ops/`.


