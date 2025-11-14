## VPN MVP Technical Plan

### Scope
- Platforms: Windows, macOS, Linux desktop clients (no mobile in this repo).
- Protocol: WireGuard via `wireguard-rs` (Rust); `wintun` crate for Windows TUN.
- Backend: Python (FastAPI) control plane with Postgres and Redis; remove Go.
- Node Plane: Rust agent (RAM-only) using `wireguard-rs`.
- Monorepo: All components orchestrated by root `docker-compose`.

### Goals and Non‑Negotiables
- Performance: Rust for data plane; multi-queue TUN; zero-copy where possible; tuned UDP stack.
- Privacy: No-logs by design; GDPR compliance; RAM-only nodes.
- Security: Strong key management; mTLS control-plane; kill switch; DNS leak protection.
- Feature Parity Targets:
  - 5 active simultaneous connections per user (enforced server-side).
  - End-to-end encryption (client↔entry/exit nodes).
  - Interconnected multi-node (dynamic mesh with peer routing) and single-server modes.
  - GDPR compliance workflows and documentation.
  - RAM-only node server software (ephemeral keys, tmpfs-only state).

### High-Level Architecture
- Clients (Rust core + Tauri UI)
  - Networking engine in Rust with `wireguard-rs`.
  - TUN adapters per OS: `wintun` (Windows), `utun`/NetworkExtension (macOS), `/dev/net/tun` (Linux).
  - Kill switch and DNS leak protection with native OS firewalls.
  - Tauri for cross-platform UI and auto-updates.
- Control Plane (Python/FastAPI)
  - `auth-api`: users, devices, sessions, policy (5-connection limit).
  - `directory-api`: node registry, regions, topology, config distribution.
  - `admin-api`: fleet control, rollout management, maintenance.
  - Postgres (source of truth) + Redis (counters, nonces, short-lived state).
  - Service-to-service mTLS; clients use OAuth2.1/OIDC-style or signed JWTs.
- Node Plane (Rust)
  - `vpn-node` agent; wireguard data plane via `wireguard-rs`.
  - Internal node mesh (WG) for multi-hop routing.
  - Control-channel gRPC to directory/control plane for configs, health, revocations.
- Observability (privacy-preserving)
  - Prometheus/Grafana with aggregate, PII-free metrics; OpenTelemetry traces (no user IDs).
- Orchestration
  - Root `docker-compose.yml` for dev/CI; production guidance via Ansible/Terraform + systemd (reference only).

### Repository Structure (Monorepo)
- `clients/`
  - `desktop/`
    - `core/` (Rust WG engine, platform abstraction)
    - `ui/` (Tauri frontend)
    - `installers/` (Windows MSI/MSIX, macOS DMG/Notarization, Linux DEB/RPM/AppImage)
- `services/`
  - `auth-api/` (Python/FastAPI)
  - `directory-api/` (Python/FastAPI)
  - `admin-api/` (Python/FastAPI)
  - `common/` (shared schemas, protobuf, pydantic models)
- `nodes/`
  - `agent/` (Rust `vpn-node` using `wireguard-rs`)
  - `egress/` (iptables/nftables templates, DNS configs)
- `infra/`
  - `docker/` (Dockerfiles)
  - `compose/` (docker-compose.yml, env templates)
  - `migrations/` (Postgres via Alembic)
  - `ansible/` (optional prod playbooks)
- `ops/`
  - `otlp/`, `prometheus/`, `grafana/` (dashboards, alerts)
- `docs/`
  - Security, privacy, architecture diagrams, DPA/GDPR docs
- `tools/`
  - CI pipelines, linters, release scripts

### Data Model (Postgres)
- `users`: id, email, password_hash, created_at, tos_version, gdpr_consent_at
- `devices`: id, user_id, device_public_key, platform, created_at, last_seen_at
- `sessions`: id, user_id, device_id, issued_at, expires_at, revoked_at, session_jti
- `nodes`: id, region, public_key, internal_wg_ip, egress_ips, capabilities, status
- `node_versions`: node_id, agent_version, wg_rs_version, updated_at
- `node_mesh_links`: from_node_id, to_node_id, allowed_routes, weight
- `connection_sessions`: id, node_id, device_id, started_at, ended_at, bytes_up, bytes_down
- `policies`: user_id, max_active_connections (default 5)
- `regions`: id, country_code, city, latency_score, status
- `audit_events`: time, type, subject_hash, payload_redacted, retention_tag

Notes:
- Do not store client IPs, destination IPs/hostnames, or DNS queries.
- Use irreversible hashing with rotating salts only when linkage is absolutely required.

### Authentication, Authorization, and 5‑Device Enforcement
- User auth: Argon2id password hashing; optional TOTP; short-lived JWT access tokens (15m), rotating refresh tokens bound to device.
- Device identity: device generates WG keypair locally; registers public key; private key never leaves device.
- Admission tokens: short-lived JWT includes `sub`, `did`, `dpk`, `exp`, policy claims.
- Node validation: offline JWT verification via JWKS; replay protection via Redis nonce/JTI.
- Active connections: Redis counters per `user_id` and `device_id` with TTL; reject >5 active connections.
- Revocation: blacklists (JTI) broadcast to nodes; forced disconnect capabilities.

### No‑Logs and GDPR by Design
- Traffic logging disabled everywhere; no per-flow metadata.
- Application logs exclude user identifiers; kernel/iptables logs disabled.
- Metrics are aggregate only (active sessions per region, total bytes per node, health).
- GDPR workflows: export/delete account; data minimization; data residency controls.
- DPA templates and Records of Processing Activities in `docs/`.

### Client Architecture (Windows, macOS, Linux)
- Rust core:
  - `wireguard-rs` session manager (keys, peers, rekey, keepalive).
  - TUN abstraction: `wintun` (Windows), `utun` + Network Extension (macOS), `/dev/net/tun` (Linux).
  - DNS override + internal DoH/DoT; IPv6 leak prevention.
  - Kill switch:
    - Windows: WFP rules allowing only TUN/control-plane.
    - macOS: NetworkExtension/PacketFilter.
    - Linux: nftables fail-closed rules.
  - Features: auto-connect, trusted networks, split tunneling (app/route), port selection, MTU/MSS tuning, multi-hop toggle.
- UI (Tauri):
  - Background service/daemon; UI triggers connect/disconnect, region/mode selection.
  - Auto-update per OS; installer flows including driver install on Windows.

### Node Architecture (Linux, RAM‑Only)
- Immutable container with `tmpfs` for `/etc/wireguard` and runtime state; swap disabled; read-only root where feasible.
- On boot: mTLS to control-plane; fetch config and mesh topology; generate ephemeral WG keys in RAM.
- Data plane: `wireguard-rs` instance, nftables NAT, tuned conntrack; multi-queue TUN; CPU pinning and IRQ tuning.
- Egress: per-region IP pools; local DNS resolver (unbound/knot) with DoT upstream and no logging.
- Security: seccomp/AppArmor; drop caps; sysctl hardening; node keys rotate on boot/daily.

### Multi‑Node Modes
- Single-server mode: client peers directly to egress node (lowest latency).
- Dynamic mesh (multi-hop): client peers to entry; traffic hops to exit via node mesh.
  - Control-plane computes path; pushes ephemeral peer configs.
  - Limit chain length to 2–3 to contain overhead.
  - Optional BGP (FRR) for large deployments (future).

### APIs and Protocols
- Client ↔ Auth: HTTPS+JSON; PKCE device-login; returns short-lived JWT; device registration with public key; DPoP-like binding.
- Client ↔ Node: WG handshake (UDP). Admission via sidecar control endpoint (local REST/gRPC) prior to bringing link up.
- Node ↔ Control-plane: mTLS gRPC streams for registration, heartbeat, config, revocations.
- Service-to-service: mTLS over gRPC; schemas in `services/common/`.

### Security and Key Management
- Root and intermediate CAs in HSM/KMS; service/node certs issue with short lifetimes.
- JWKS for client admission tokens; rotate keys regularly (hours).
- Replay prevention: nonce/JTI with short TTL; strict clock skew bounds.
- Supply chain: reproducible builds, SBOM, signed artifacts, update signature verification.

### Performance Plan
- Multi-queue TUN with worker pools; lock-free ring buffers.
- Zero-copy TUN↔UDP where possible; `tokio` with IOCP/kqueue/epoll per OS.
- Batching (GSO/GRO), adaptive MTU/MSS clamping.
- CPU pinning of hot paths; NUMA awareness; RPS/XPS tuned.
- Adaptive keepalive; fast reconnect; mobility resilience.
- Benchmark harness with iperf3 and synthetic packet generators; CI performance gates.

### Docker Compose (Dev Skeleton)
```yaml
version: "3.9"
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: example
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck: { test: ["CMD-SHELL", "pg_isready -U postgres"], interval: 5s, retries: 10 }

  redis:
    image: redis:7
    command: ["redis-server", "--appendonly", "no"]
    healthcheck: { test: ["CMD", "redis-cli", "ping"], interval: 5s, retries: 10 }

  auth-api:
    build: ./services/auth-api
    env_file: ./services/auth-api/.env
    depends_on: [postgres, redis]
    ports: ["8080:8080"]

  directory-api:
    build: ./services/directory-api
    env_file: ./services/directory-api/.env
    depends_on: [postgres, redis, auth-api]
    ports: ["8081:8081"]

  admin-api:
    build: ./services/admin-api
    env_file: ./services/admin-api/.env
    depends_on: [auth-api, directory-api]
    ports: ["8082:8082"]

  node-agent:
    build: ./nodes/agent
    cap_add: ["NET_ADMIN", "SYS_MODULE"]
    sysctls:
      - net.ipv4.ip_forward=1
      - net.ipv6.conf.all.forwarding=1
    tmpfs:
      - /etc/wireguard:rw,noexec,nosuid,nodev,mode=0700
      - /var/run/vpn:rw,noexec,nosuid,nodev,mode=0700
    depends_on: [directory-api]
    network_mode: "host" # dev only

volumes:
  pgdata:
```

### Testing Strategy
- Unit tests: auth, token issuance/validation, device registration.
- Integration: bring up compose; simulate node+clients; enforce 5-device limit.
- Performance: lab with 1/10/40 Gbps; CPU profiles; packet loss/jitter scenarios.
- Chaos: node failures, key rotation mid-session, Redis/Postgres outages.
- Privacy: scan logs/metrics for PII; red-team de-correlation attempts.

### Packaging and Distribution
- Windows: MSI/MSIX; signed `wintun` driver; service + UI.
- macOS: Universal binary; NetworkExtension entitlements; notarization; Sparkle updates.
- Linux: DEB/RPM/AppImage; polkit for privilege elevation.

### Risks and Mitigations
- User-space WG throughput < kernel WG:
  - CPU pinning, batching, multi-queue TUN, NIC tuning, and continuous perf testing.
- Kill switch complexity across OSes:
  - Invest in native integrations; robust fail-closed and recovery paths; extensive tests.
- RAM-only nodes reduce forensics:
  - Strengthen aggregate telemetry and live-debug tooling without PII.
- Multi-hop adds latency:
  - Default single-hop; clear UX and guidance; smart selection for multi-hop.

### Phased Roadmap
- Phase 0: Prep
  - Remove Go services; establish monorepo layout; CI; formatters/linters (ruff, mypy; clippy; cargo fmt).
- Phase 1: Control-plane MVP
  - `auth-api` (users/devices/sessions/JWT/JWKS/refresh) + Redis.
  - `directory-api` (node registry, regions, config streaming).
  - Alembic migrations; seed scripts.
- Phase 2: Node Agent
  - Rust agent with mTLS bootstrap; ephemeral WG keys; single-server mode.
  - NAT/firewall templates; DNS (unbound) RAM-only; admission + counters + reaper.
- Phase 3: Desktop Client
  - Rust core engine; TUN adapters; kill switch; DNS protection.
  - Tauri UI; installers; Windows driver flow.
- Phase 4: Mesh + Multi-hop
  - Internal node WG mesh; path selection; config distribution.
  - Client UX for multi-hop toggle.
- Phase 5: Hardening & Performance
  - System tuning; benchmarking; security review and pen-test.
- Phase 6: Compliance & Ops
  - GDPR processes; DPA; data export/delete automation; observability with SLOs.

---

## Workstreams and Delegation (Independent Sections)
Each workstream below is designed to be owned by a dedicated agent with minimal cross-dependencies. Cross-team contracts are defined via schemas and APIs in `services/common/` and `docs/`.

### WS1: Monorepo, CI/CD, and Compose Orchestration
Ownership: Infra Agent
- Define repo structure and scaffolds.
- Author root `docker-compose.yml` and `.env` templates.
- Set up CI (lint, test, build) for Python, Rust, and Tauri.
- Containerize services and node agent; dev vs prod profiles.
- Provide developer docs and make targets.
Dependencies: none (foundational).

### WS2: Auth and User Management (`services/auth-api`)
Ownership: Backend Auth Agent
- User registration/login with Argon2id; TOTP optional.
- Device registration with device public keys.
- JWT access tokens + rotating refresh tokens; JWKS rotation.
- GDPR endpoints: export/delete; consent tracking.
- Alembic migrations; Postgres models; Redis integration.
Dependencies: WS1 (compose, DB).
Contracts: JWT/JWKS schemas; device registration API.

### WS3: Directory and Topology (`services/directory-api`)
Ownership: Backend Directory Agent
- Node registry and health; region catalog.
- Config distribution via gRPC streams (mTLS).
- Admission policy broadcast and revocation lists.
- Mesh topology management and route advertisements.
Dependencies: WS1; interoperates with WS2 for token verification keys.
Contracts: Node registration/heartbeat/config protobufs.

### WS4: Admin and Fleet Control (`services/admin-api`)
Ownership: Backend Admin Agent
- Node lifecycle management; rollout/versioning endpoints.
- Maintenance windows; feature flags.
- Audit trail (redacted) and operator auth.
Dependencies: WS1–WS3.
Contracts: Admin APIs; role model.

### WS5: Node Agent (`nodes/agent`)
Ownership: Node Agent Engineer (Rust)
- mTLS bootstrap; ephemeral WG key management.
- Single-server data plane using `wireguard-rs`.
- NAT, firewall (nftables) templates; DNS (unbound) integration.
- Admission token validation; active connection counters; disconnect reaper.
- Heartbeats and config application; RAM-only operation.
Dependencies: WS3 (config, keys); WS1 (compose).
Contracts: Protobufs from `services/common/`.

### WS6: Desktop Client Core (`clients/desktop/core`)
Ownership: Client Core Engineer (Rust)
- WireGuard session manager; peer management; rekey/keepalive.
- TUN abstraction per OS: `wintun`, `utun`/NE, `/dev/net/tun`.
- Kill switch and DNS leak protection per OS.
- Split tunneling; auto-connect; MTU/MSS tuning; port selection.
Dependencies: WS2 (auth flows), WS5 (node expectations).
Contracts: Auth API; node admission sidecar protocol.

### WS7: Desktop UI and Installers (`clients/desktop/ui` and `installers/`)
Ownership: Client UI/Release Engineer
- Tauri UI: onboarding, login, region/mode selection, status.
- Background service/daemon integration.
- Auto-updates; installers per OS; Windows driver install flow.
Dependencies: WS6 (core APIs), WS2 (auth UX).
Contracts: Core command interfaces; update/signing pipeline.

### WS8: Mesh and Multi‑Hop (Node + Client)
Ownership: Mesh Engineer (Rust + Backend)
- Internal WG mesh across nodes; entry/exit selection logic.
- Control-plane path computation; ephemeral peer config distribution.
- Client UX toggle for single vs multi-hop.
Dependencies: WS3, WS5, WS6, WS7.
Contracts: Mesh config schemas; client toggle API.

### WS9: Security, Compliance, and Privacy
Ownership: Security/Compliance Lead
- Key management policies; CA hierarchy; JWKS rotation process.
- No-logs enforcement; log redaction; retention controls.
- GDPR DPA, ROPA, data export/delete automation.
- Threat modeling; pen-test coordination; supply-chain hardening.
Dependencies: all; advisory across workstreams.

### WS10: Observability and SRE
Ownership: SRE/Observability Engineer
- Prometheus + Grafana; privacy-preserving metrics.
- OpenTelemetry in services (no user IDs); alerts and SLOs.
- On-call runbooks; incident response (privacy-first).
Dependencies: WS1–WS4; supports WS5–WS8.

---

## Contracts and Interfaces
- Schemas: Protobuf and OpenAPI definitions live in `services/common/` and are versioned.
- Backward compatibility: additive changes; coordinated removals through feature flags.
- Security: mTLS everywhere in control/data plane control paths; admission JWTs short-lived.

## Compliance Notes
- Data minimization, purpose limitation, and retention controls are enforced by design.
- Regional data residency is configurable; no unintended cross-region movement of PII.
- User data export and deletion are complete and verified in integration tests.

## Next Concrete Steps
- Initialize monorepo structure and compose stack (WS1).
- Scaffold `auth-api` (users/devices/sessions/JWT/JWKS) and migrations (WS2).
- Scaffold `directory-api` (registry, heartbeat, config stubs) (WS3).
- Scaffold `node-agent` with no-op WG bring-up and heartbeat (WS5).
- Build client core CLI harness to connect to a dev node (WS6).


