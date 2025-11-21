## VPN MVP Overview

This repository contains everything required to build a privacy-first, multi-platform VPN product. The monorepo was first designed as a simple test for an LLMs capability of planning our major infrastructure.

---

## Product Philosophy
- **Performance as a feature**: Rust for anything that moves packets, tight control over multi-queue TUN, and benchmarking in CI keep latency low and throughput high.
- **Privacy by default**: No traffic logs, RAM-only nodes, GDPR workflows, and privacy-preserving telemetry ensure we never need to ask users for trust—we earn it.
- **Secure roots-of-trust**: Strong key management (mTLS, JWKS rotation, HSM-backed CA) and short-lived, scoped credentials make compromise expensive and containment fast.
- **Respect for the desktop**: Local-first clients with kill switches, DNS protection, and proper installer/auto-update flows per OS provide a native experience rather than a repackaged browser.
- **Sane scope**: Desktop only (Windows, macOS, Linux) for this MVP; mobile is out-of-repo by design to keep the initial surface focused.

---

## System at a Glance
- **Clients**: Rust networking core plus a Tauri UI. Each platform ships with native TUN plumbing, kill switch logic, and OS-appropriate installers.
- **Control Plane**: FastAPI services backed by Postgres and Redis. `auth-api` issues and revokes access, `directory-api` manages regions and topology, `admin-api` handles fleet operations.
- **Node Plane**: A Rust agent running WireGuard in user space, operating fully in RAM with ephemeral keys. Nodes form both single-hop gateways and optional multi-hop meshes.
- **Observability & Ops**: Prometheus/Grafana and OpenTelemetry traces capture aggregate health without leaking user data. Automation lives alongside infra definitions (compose, Ansible/Terraform samples) for reproducible environments.

---

## Repository Map
- `clients/desktop`: Rust core, Tauri UI, and platform-specific installer assets.
- `services`: FastAPI services plus shared schemas in `services/common`.
- `nodes`: Node agent code, egress templates, and mesh plumbing.
- `infra`: Dockerfiles, compose files, migrations, and deployment references.
- `ops`: Observability configs, runbooks, and SLO definitions.
- `docs`: Security/privacy references, architecture diagrams, compliance artifacts.
- `tools`: CI helpers, chaos harnesses, and release automation.

---

## Guiding Principles for Delivery
- **Single source of truth**: Contracts (protobuf/OpenAPI) live in-repo and are versioned; all teams consume them via generated artifacts.
- **Incremental hardening**: Every phase ends with usable software; later phases layer mesh routing, performance tuning, and compliance automation without blocking earlier milestones.
- **Tight feedback loops**: Root `docker-compose.yml` spins up the entire stack for local testing, and chaos/testing tools live beside the code they verify.
- **Composability**: Workstreams own clear surfaces but converge on shared infra (CI, compose, observability) so teams can ship independently.

## Roadmap Highlights
1. **Phase 0 – Foundations**: Finalize repo scaffolding, compose stack, baseline CI, and shared coding standards.
2. **Phase 1 – Control Plane MVP**: Ship `auth-api` and `directory-api` with core Postgres/Redis migrations, token issuance, and node registry.
3. **Phase 2 – Node Agent Alpha**: Bring up the Rust node agent in single-hop mode with admission tokens, RAM-only storage, and heartbeat reporting.
4. **Phase 3 – Desktop Client Alpha**: Deliver the networking core + basic Tauri UI, wired to a dev node for connect/disconnect flows and kill-switch enforcement.
5. **Phase 4 – Mesh & Multi-hop**: Introduce multi-node routing, client UX toggles, and control-plane orchestration for path selection.
6. **Phase 5 – Hardening & Performance**: Benchmark, tune, and add chaos/perf tests; perform threat modeling and security reviews.
7. **Phase 6 – Compliance & Ops**: Complete GDPR workflows, finalize DPA/ROPA docs, and stand up SLO-based observability.

---

## Tenets for Contributors
- **Default secure**: Every new feature must articulate its impact on keys, secrets, and privacy before landing.
- **Test where it hurts**: Prefer integration tests that exercise full flows (auth → node → client) to catch regressions early.
- **Document contracts**: Update `docs/` and `services/common/` alongside code changes; humans rely on those narratives to operate the system safely.
- **Benchmark before optimizing**: Use the shared perf harness to prove wins; anecdotal speedups rarely hold across platforms.
- **Own the rollback story**: Container images, installers, and migrations all need reversible paths; build them in from the start.

---

## Getting Started
1. Install Rust, Python 3.11+, Node 18+, and Docker.
2. Run `make bootstrap` to install toolchains and pre-commit hooks.
3. Launch the full stack with `docker compose up` from the repo root; it seeds Postgres/Redis and starts FastAPI services plus a dev node.
4. Point the desktop client at the local control plane using `.env.local` templates under `clients/desktop`.
5. Use `tools/chaos` and `TESTING.md` to run targeted chaos, load, and privacy checks.

For detailed architecture, compliance, and operational docs, see the [`docs/`](docs/INDEX.md) index.

Additional guides on testing and ops are in [`docs/`](docs/). Start there for deep dives on privacy, threat modeling, or compliance posture.


---

## Why This Matters
Combining a Rust data plane, privacy-first control loops, and deliberate desktop UX raises the bar for consumer VPNs. Success means shipping a fast, trustworthy network whose operations are explainable and auditable. Every file in this repository is in service of that outcome: minimal trust surface, transparent automation, and a product philosophy anchored on user safety over vanity metrics.


