VPN MVP – Deployment Readiness Review and Fault Log
===================================================

Scope
-----
- Monorepo: Rust clients and node agent, Python control‑plane (`auth-api`, `directory-api`, `admin-api`), shared `services/common`, and root `docker-compose.yml`.
- Focus: Security, privacy, performance, reliability, and deployability. All findings include impacted components, severity, rationale, and concrete remediation guidance.
- Status: Not deployment‑ready. Critical security/auth gaps, migrations/DB driver mismatch, module packaging issues, and supply‑chain hardening gaps block release.

Executive Summary
-----------------
- Critical issues:
  - Insecure gRPC control channel in `directory-api` (no TLS/mTLS).
  - Unauthenticated/missing authorization on `directory-api` HTTP routes (mutations exposed).
  - `auth-api` uses `postgresql://` DSN expecting `psycopg2`, but image installs `psycopg` v3; container fails to start.
  - `directory-api` imports from `services.common` without copying the package into its image; container fails to start.
  - DB migrations are not executed on container startup for `auth-api`.
- High risks:
  - JWKS rotation uses ephemeral in‑memory keys in `auth-api` (rotation on restart breaks verification).
  - No rate limiting or brute‑force throttling on auth/device flows.
  - Images not hardened, dependencies not pinned; no SBOM/signing.
  - Node agent uses `network_mode: "host"` in compose (dev‑only but dangerous if reused).
- Privacy posture:
  - Good “no logs” intent; partial PII scrubbing in observability (allowlist is too narrow).
- Performance:
  - Acceptable for MVP; some indexing and async consistency improvements recommended.

Critical Findings (Must Fix Before Production)
----------------------------------------------
1) Directory gRPC control channel is insecure (no TLS/mTLS)
- Location: `services/directory-api/app/grpc_server.py` – `_server.add_insecure_port(address)` with comment “dev only”.
- Impact: Anyone able to reach the port can connect, observe config streams, and impersonate nodes. Enables rogue nodes and control‑plane interception.
- Severity: Critical.
- Remediation:
  - Require TLS using `grpc.ssl_server_credentials` and client certificate validation.
  - Bind to non‑public interface; terminate at mTLS sidecar (Envoy/h2) or at app directly.
  - Authenticate peer identity (SAN/SPIFFE) on every RPC.

2) Directory HTTP routes lack authentication/authorization
- Location: `services/directory-api/app/routers/*.py` (e.g., `regions.py`, `nodes.py`, `mesh.py`) – routes accept requests with no auth dependency.
- Impact: Arbitrary callers can register/heartbeat nodes, mutate regions, or apply mesh paths.
- Severity: Critical.
- Remediation:
  - Require bearer JWT validated against `auth-api` JWKS for all write endpoints.
  - Enforce RBAC for operator‑only routes; use mTLS + admission JWT for node‑originated requests.

3) `auth-api` Postgres driver mismatch; container fails to start
- Location:
  - DSN: `services/auth-api/app/config.py` sets `DATABASE_URL = "postgresql://..."` (defaults to `psycopg2`).
  - Package: `services/auth-api/requirements.txt` installs `psycopg[binary]` (psycopg v3).
- Impact: ModuleNotFoundError for `psycopg2` on startup; `auth-api` cannot start.
- Severity: Critical.
- Remediation:
  - Switch DSN to `postgresql+psycopg://...` or install `psycopg2-binary`.

4) `directory-api` missing shared package in container
- Location:
  - Import: `services/directory-api/app/main.py` → `from services.common.observability import init_observability`
  - Dockerfile copies only `app/` into the image.
- Impact: ImportError (`No module named 'services'`); container fails to start.
- Severity: Critical.
- Remediation:
  - Copy `services/common/` into the image or package it (wheel) and install at build time.
  - Alternatively, vendor `observability` module inside the service.

5) Database migrations not executed for `auth-api`
- Location: `services/auth-api/Dockerfile`; no Alembic invocation.
- Impact: Fresh deployments lack schema; runtime failures or undefined behavior ensue.
- Severity: Critical.
- Remediation:
  - Run `alembic upgrade head` on container start (entrypoint) or via an init job.
  - Ensure proper DB URL for Alembic with `psycopg` or `psycopg2`.

High‑Risk Issues
----------------
6) JWKS rotation uses ephemeral in‑memory keys
- Location: `services/auth-api/app/security.py` (`KeyManager`).
- Impact: Restart rotates signing keys; tokens issued previously may be unverifiable by other services, breaking SSO and inter‑service auth.
- Severity: High.
- Remediation:
  - Persist active signing keys in KMS/HSM or sealed storage.
  - Make rotation explicit and coordinated (publish new JWKS before activation; maintain overlap).

7) Missing rate limiting/brute-force mitigations on login/device flows
- Location: `services/auth-api/app/routers/auth.py`.
- Impact: Attackers can brute-force passwords/TOTP/device auth endpoints.
- Severity: High.
- Remediation:
  - Add per‑IP/user rate limiting via Redis sliding window; exponential backoff.
  - Optional CAPTCHA/step‑up after repeated failures; audit events for repeated failures.

8) Supply chain and image hardening gaps
- Location: Python service Dockerfiles and requirements.
- Impact: Non‑reproducible builds; larger attack surface; runtime packages include build tools.
- Severity: High.
- Remediation:
  - Pin exact versions (==), generate lockfiles (pip‑tools), and produce SBOM (CycloneDX).
  - Use multi‑stage builds, non‑root users, read‑only FS, drop capabilities, seccomp/AppArmor.
  - Sign images and verify at deployment.

9) Node agent uses host networking in compose
- Location: `docker-compose.yml` → `network_mode: "host"` on `node-agent`.
- Impact: Acceptable in dev; unsafe if propagated to prod. Port leaks and host interference possible.
- Severity: High (prod), Low (dev).
- Remediation:
  - Gate with a dev profile only; add CI guard to prevent prod use.

Medium/Lower‑Risk Issues
------------------------
10) Observability PII scrubbing is allowlist‑based and partial
- Location: `services/common/observability/__init__.py` – `SENSITIVE_ATTR_KEYS` set is narrow.
- Impact: PII can leak via non‑enumerated attributes/baggage.
- Severity: Medium.
- Remediation:
  - Default‑deny span attributes with an explicit allowlist; add tests that scan exported spans/metrics for PII and fail CI if found.

11) Directory gRPC stubs generated at runtime
- Location: `services/directory-api/app/grpc_server.py` using `grpc_tools.protoc` at startup.
- Impact: Larger runtime images, increased cold‑start latency; couples runtime and build toolchains.
- Severity: Medium.
- Remediation:
  - Pre‑generate gRPC stubs at build time and ship them (commit to repo or installable wheel).

12) `auth-api` synchronous SQLAlchemy usage (throughput ceiling)
- Location: `services/auth-api/app/db.py` (sync engine).
- Impact: Potential worker starvation under load; inconsistent with other async services.
- Severity: Medium.
- Remediation:
  - Migrate to SQLAlchemy asyncio for consistency and scalability.

13) Compose lacks app healthchecks
- Location: `docker-compose.yml` – healthchecks defined for Postgres/Redis only.
- Impact: Orchestration may start dependencies too early; flakier local/CI flows.
- Severity: Medium.
- Remediation:
  - Add HTTP healthchecks for `auth-api`, `directory-api`, `admin-api` and use `depends_on.condition: service_healthy`.

14) Tests rely on environment side‑effects and compose availability
- Location: `tests/server/directory_api/test_api.py` vs `services/directory-api/tests/test_api.py`.
- Impact: Confusing CI behavior when DB/Redis are not present; tighter isolation needed for unit tests.
- Severity: Medium.
- Remediation:
  - Use `sqlite://` and `fakeredis` for unit tests; reserve real Postgres/Redis for integration tests in compose.

Privacy Review
--------------
- Positive:
  - Data models avoid storing traffic/DNS metadata. GDPR intent and residency controls noted in docs.
  - JWKS/short‑lived access tokens planned; no per‑flow logs.
- Risks:
  - Observability scrubbing is partial (see item 10).
  - Ephemeral key rotation (item 6) risks ad hoc logging during incidents; fix rotation/persistence to avoid pressure to log sensitive data.
  - Ensure `admin-api` audit events redact identifiers consistently (spot‑check models/logging).

Performance Review
------------------
- Directory DB:
  - Queries order by `last_seen_at`; add indexes on `(region_id, status, last_seen_at)` (or appropriate composite keys) for common listings/filters.
  - Unique index present on node public keys (good).
- Config streaming:
  - In‑process asyncio queues with maxsize and drop‑newest policy; monitor drop metrics; consider merge/backoff on overflow.
- Auth DB:
  - Sync SQLAlchemy likely fine for MVP but caps throughput; consider async alignment with other services.

Operational and Orchestration Review
------------------------------------
- Compose:
  - Good dev coverage (Postgres/Redis/Prometheus/Grafana/Jaeger/Alertmanager).
  - Missing service healthchecks for app containers.
  - No centralized migration job; add init flow or service entrypoints to run Alembic.
- Secrets/config:
  - `.env` pattern for dev. For prod, move to external secret managers and mTLS materials from secure stores.
- Monitoring:
  - Aggregate metrics only (good). Keep cardinality low; ensure any labels avoid PII (user IDs/emails).

Supply Chain & Reproducibility
------------------------------
- Requirements files use `>=` version specifiers; images install build‑time tools into runtime layers.
- Recommendations:
  - Pin and lock dependencies; multi‑stage builds producing slim runtime images; SBOM generation and image signing; non‑root users; read‑only root FS and seccomp profiles.

End‑to‑End Test Plan (Executed)
-------------------------------
Planned sequence:
1) Rust workspace build and tests: `cargo build --workspace && cargo test --workspace`
2) Compose control‑plane services: `docker compose up -d --build postgres redis auth-api directory-api admin-api`
3) Verify container health; collect logs upon failures.
4) Service unit/integration tests:
   - Inside services or host with `pytest` (unit: no external deps; integration: against compose).
5) E2E flows: auth (register/login/JWKS), directory (register node/region CRUD), admin (operator RBAC), basic mesh stub calls.

E2E Fault Log (Results)
-----------------------
This section captures faults observed during automated bring‑up and tests. Each includes error, root cause, and remediation.

1) `auth-api` container crash on startup
- Error: `ModuleNotFoundError: No module named 'psycopg2'`
- Root cause: DSN `postgresql://...` implies `psycopg2`, but image installs `psycopg` v3 (`psycopg[binary]`).
- Impact: `auth-api` never starts; control‑plane unusable.
- Remediation: Use `postgresql+psycopg://...` DSN or install `psycopg2-binary`.

2) `directory-api` container crash on startup
- Error: `ModuleNotFoundError: No module named 'services'` when importing `services.common.observability`.
- Root cause: Dockerfile does not copy/install `services/common/` into the image.
- Impact: `directory-api` never starts.
- Remediation: Copy and install shared package, or vendor needed module.
- Note: gRPC also runs insecurely and should be mTLS‑protected (Critical Finding #1).

3) `admin-api` route registration assertion (to verify)
- Symptom (previously observed pattern): `AssertionError: Status code 204 must not have a response body`.
- Status: Current `DELETE /operators/{id}` returns `None` (no body). If assertion recurs, ensure no `response_model` or body is returned with 204.
- Remediation: Keep 204 with no body, or change to 200 with a minimal response.

4) `node-agent` image build failure (environment dependent)
- Error: `Dockerfile parse error: COPY cannot accept a heredoc as a destination` (older builders).
- Root cause: Dockerfile uses syntax not supported by the builder variant.
- Remediation: Avoid heredoc/COPY tricks; use standard `COPY` with files or switch BuildKit/syntax version.

5) Rust workspace build misconfiguration
- Error: `failed to read ... \\clients\\client\\vpn-core\\Cargo.toml` (path does not exist) when running `cargo build --workspace`.
- Root cause: Workspace member or dependency path in `Cargo.toml` points to `clients/client/vpn-core` instead of the actual `client/vpn-core` or `clients/desktop/...` layout.
- Impact: Rust workspace cannot build/tests cannot run; blocks client/UI service readiness.
- Remediation: Fix workspace member paths in root `Cargo.toml` and any `path` dependencies to align with current repo structure; verify `cargo test --workspace` passes in CI.

Deployment Readiness Verdict
----------------------------
- Current state: NOT READY for customer deployment.
- Blocking items to resolve (minimum):
  1) Secure directory gRPC with TLS/mTLS; enforce auth on all directory mutations.
  2) Fix `auth-api` DSN/driver; run Alembic migrations automatically.
  3) Package/install `services/common` for `directory-api`.
  4) Pin dependencies; harden images; add SBOM/signing.
  5) Add service healthchecks and standardized test strategy (unit vs integration).
  6) Add rate limiting on auth/device flows and persist signing keys.

Concrete Remediation Plan (Ordered)
-----------------------------------
1) Enforce auth on `directory-api` HTTP routes and add TLS/mTLS to gRPC.
2) Add Alembic migrations execution to `auth-api` container entrypoint; update DSN to `postgresql+psycopg://`.
3) Package and install `services/common` into service images; stop runtime proto generation and bake stubs.
4) Lock dependencies, add non‑root users, minimal runtime images, SBOMs, and image signing.
5) Add rate limiting/backoff for login and device flows; consider CAPTCHA after N failures.
6) Harden observability: deny‑by‑default span attributes; add CI tests that fail on PII in spans/metrics.
7) Add app healthchecks in compose and use health‑based dependencies.
8) Add integration test harness to seed DBs and exercise E2E flows under compose.

Appendices
----------
- Key files validated:
  - `services/directory-api/app/grpc_server.py` (insecure gRPC; runtime stub generation)
  - `services/directory-api/app/main.py` (imports from `services.common`)
  - `services/directory-api/app/routers/{regions,nodes,mesh}.py` (no auth on mutations)
  - `services/auth-api/app/config.py` (DSN driver mismatch)
  - `services/auth-api/app/db.py` (sync SQLAlchemy)
  - `services/auth-api/app/security.py` (ephemeral JWKS manager)
  - `services/common/observability/__init__.py` (PII scrubbing allowlist)
  - `docker-compose.yml` (host networking for `node-agent`, missing app healthchecks)
  - `services/*/requirements.txt` (unpinned versions)

Change Log
----------
- Initial comprehensive review and fault log created to guide remediation and future agent work.


