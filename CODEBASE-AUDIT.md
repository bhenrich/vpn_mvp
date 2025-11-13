VPN MVP – Codebase Audit and Deployment Readiness

Overview

- Scope: Full review of the monorepo per PLAN.md focusing on security, privacy, performance, reliability, and deployability of the Python control-plane services (auth-api, directory-api, admin-api) plus orchestration via docker-compose. Client and node-agent code were noted but not exercised end-to-end in this pass.
- Status: Architectural scaffolding is strong and aligns with PLAN.md. Several critical deployment blockers remain (detailed below), chiefly around security hardening, authentication/mTLS in inter-service paths, database migrations, and reproducible builds.
- Outcome: This document enumerates issues by severity with rationale, impact, and concrete remediation guidance. E2E test faults will be appended at the end after execution.

Architecture Conformance (from PLAN.md)

- Control plane: FastAPI-based services for auth, directory, admin with Postgres and Redis present in root docker-compose.yml. Good.
- Observability stack (Prometheus, Grafana, Jaeger/OTLP) wired in compose. Good; dev-ready.
- Node plane and client scaffolding exist; mesh/config streaming stubs present in directory-api via gRPC. Good for MVP.
- GDPR/no-logs principles are reflected in data models (no storage of traffic/DNS). Good direction.

Critical Findings (must-fix before production)

1) No mTLS/TLS on gRPC control channel (directory-api)

- Location: services/directory-api/app/grpc_server.py (add_insecure_port on 0.0.0.0:grpc_port, comment “dev only; mTLS to be added later”).
- Impact: Any party able to reach the port can open streams, receive configs, and observe control-plane activity. Enables rogue nodes or interception.
- Risk: Critical – control channel compromise.
- Remediation:
  - Require TLS with client certificate auth: add server credentials via grpc.ssl_server_credentials with CA bundle and node-issued certs.
  - Bind to internal network only in dev; use sidecar mTLS proxy (envoy/h2) or direct gRPC TLS in prod.
  - Gate all RPC methods on authenticated peer identity (SPIFFE/SAN checks).

2) Directory HTTP endpoints lack authentication/authorization

- Location: services/directory-api/app/routers/*.py (nodes, regions, mesh) – all routes are publicly callable.
- Impact: Arbitrary node registration/updates, path selection, and config pushes are possible without tokens or roles.
- Risk: Critical – registry poisoning/config abuse.
- Remediation:
  - Require bearer tokens validated against auth-api JWKS for all write endpoints; add RBAC for operator-only paths.
  - For node-originating endpoints (register/heartbeat), require mTLS at edge and/or admission JWT tied to node identity.

3) Database migrations not executed for auth-api (deployment blocker)

- Location: services/auth-api/Dockerfile and code paths – no Alembic invocation on startup. auth-api uses sync SQLAlchemy engine without auto-create.
- Impact: Fresh deployments will start with empty schema; auth-api will fail or behave incorrectly.
- Risk: Critical – service won’t function in real deploys.
- Remediation:
  - Add entrypoint or startup hook to run Alembic migrations (alembic upgrade head) before app serve.
  - Ensure proper SQLAlchemy URL for migrations (psycopg3) and env var exposure in container.

4) JWKS rotation uses ephemeral in-memory keys with no persistence

- Location: services/auth-api/app/security.py (KeyManager).
- Impact: Service restarts rotate signing keys non-deterministically; previously issued tokens can become unverifiable, breaking SSO and inter-service auth.
-.Risk: High – auth instability; outages after restarts/rollouts.
- Remediation:
  - Persist active/private keys (KMS/HSM or sealed storage). Rotation should be explicit and coordinated.
  - Prefer EC P-256/Ed25519 where applicable; consider 3072/4096 RSA if RSA required.

5) Missing rate limiting and password/TOTP brute-force protections

- Location: services/auth-api/app/routers/auth.py (login, device flows).
- Impact: Attackers can brute-force credentials and device authorization endpoints.
- Risk: High – account takeover risk and abuse.
- Remediation:
  - Add IP/user-based rate limiting (e.g., Redis sliding window) and exponential backoff.
  - Consider CAPTCHA/step-up challenges on repeated failures.

6) Supply chain / image hardening gaps

- Location: Python Dockerfiles (services/*/Dockerfile) and requirements.txt files.
- Impact: Non-reproducible builds and larger attack surface.
- Risk: High – reliability and security exposure.
- Remediation:
  - Pin exact versions (==) for all runtime deps, freeze lock (pip-compile) and produce SBOM.
  - Create non-root user, drop capabilities, and use distroless/ubi-micro where possible.
  - Add runtime seccomp/AppArmor profiles and read-only FS where feasible.

7) Node-agent uses host networking in compose (dev-only) but not walled

- Location: docker-compose.yml (network_mode: host for node-agent).
- Impact: Easy to leak ports and interfere with host; acceptable in dev but must be forbidden in prod.
- Risk: High in prod if reused; Low in dev as-is.
- Remediation:
  - Gate host networking behind dev profile only; add explicit warning and CI guard against prod use.

8) Admin bootstrap path can silently grant admin if env var set

- Location: services/admin-api/app/main.py (bootstrap_operator_user_id).
- Impact: Misconfiguration may surprise-elevate a user to admin.
- Risk: Medium.
- Remediation:
  - Require signed/one-time bootstrap tokens or migration-time initialization; log explicit, auditable events.

9) Observability PII scrubbing is allowlist-based and partial

- Location: services/common/observability/__init__.py (SENSITIVE_ATTR_KEYS limited set).
- Impact: PII may leak in attributes or baggage not covered by keys.
- Risk: Medium.
- Remediation:
  - Switch to deny-by-default/allowlist of safe attributes; scrub request/response payload sampling; ensure no emails/user identifiers in spans or logs. Add tests to enforce.

10) Auth-api synchronous DB access (performance and scalability)

- Location: services/auth-api/app/db.py (sync engine, pooled).
- Impact: For higher throughput, async can reduce worker starvation; current is acceptable for MVP but a ceiling exists.
- Risk: Medium (performance/scale).
- Remediation:
  - Consider async stack (SQLAlchemy asyncio) to align with other services for consistency and throughput.

11) Directory-api gRPC stub generation at runtime

- Location: services/directory-api/app/grpc_server.py (ensure_protos_compiled with grpc_tools.protoc at startup).
- Impact: Adds runtime complexity, increases cold-start latency, and couples runtime image with build toolchain.
- Risk: Medium.
- Remediation:
  - Pre-generate stubs in build phase and bake into the image; verify integrity at build-time.

12) Authentication missing on directory-api region CRUD

- Location: services/directory-api/app/routers/regions.py (public CRUD; not shown here but included by app).
- Impact: Untrusted callers can modify region catalog; cascades into path selection and node affinity.
- Risk: Critical (same family as #2).
- Remediation:
  - Require operator/admin JWT; no public access for mutations.

13) Redis lifecycle and error handling

- Location: services/auth-api/app/dependencies.py (singleton client without shutdown), directory-api startup hooks (init/close OK).
- Impact: Leaked connections or silent failures in error paths.
- Risk: Low/Medium.
- Remediation:
  - Add shutdown lifecycle for auth-api similar to directory-api; add retries/backoff and circuit-breakers in hot paths.

14) Tests rely on environmental side-effects

- Location: tests/server/directory_api/test_api.py vs services/directory-api/tests/test_api.py.
- Impact: The root tests will execute startup hooks expecting Postgres/Redis availability; they will fail outside compose. This makes CI confusing if not orchestrated.
- Risk: Medium (DX/CI stability).
- Remediation:
  - Standardize: run service unit tests with startup hooks disabled or use sqlite/embedded redis (fakeredis) in unit context; reserve integration tests to compose.

Privacy Review

- Positive: No storage of traffic/DNS metadata; models limited to User/Device/Policy/session with necessary data only; residency captured; responses avoid echoing PII beyond email when required.
- Risks:
  - Observability scrubbing is partial (see #9).
  - JWKS/key handling not persisted (see #4) could lead to emergency diagnostics pressure and ad-hoc logging—avoid by fixing rotation.
  - Admin API must ensure audit trail redacts identifiers consistently (not fully reviewed here; verify in app/models and any logging).

Performance Review

- Directory DB access:
  - Queries order by last_seen_at; consider indexes on (region_id, status, last_seen_at) for fast selection and node listings.
  - Unique index exists on nodes.public_key; good.
- Mesh config streaming:
  - In-process asyncio queues keyed by public_key with backpressure; reasonable for MVP. Monitor drop behavior on overflow (QueueFull → drops newest update). Consider bounded retries/merging.
- Auth-api sync DB may cap throughput (see #10).

Operational/Orchestration Review

- docker-compose.yml is comprehensive for dev, includes observability; good for local/integration.
- Missing explicit healthchecks for app services (only Postgres/Redis defined). Add HTTP healthchecks with depends_on: condition: service_healthy.
- No centralized migrations step; add either “init” jobs or service entrypoints to run Alembic.
- Secrets: services use .env files; do not commit real secrets. For prod: external secret management (Vault/KMS) and mTLS materials.

Reproducibility and Supply Chain

- Python requirements use >= for many dependencies; not reproducible.
- Docker images run as root and install build-time tools in runtime image.
- Recommendations:
  - Pin versions and lock, produce SBOM (CycloneDX), and sign images.
  - Multi-stage builds: build wheels and keep runtime minimal; run as non-root; drop capabilities.

Deployment Readiness – Checklist

- mTLS/TLS on all service-to-service and control channels: NOT READY (see #1, #2).
- AuthN/Z enforcement on mutating APIs: NOT READY (directory-api unauthenticated).
- DB migrations automated and idempotent: NOT READY (auth-api).
- Reproducible builds with pinned deps and non-root images: NOT READY.
- Healthchecks, readiness gates, and observability PII guardrails: PARTIAL.
- Rate limiting and auth hardening: NOT READY.

Conclusion

- The codebase is architecturally aligned and close for an MVP, but it is not deployment-ready yet due to critical security/auth gaps, missing migrations automation, and supply chain hardening.

Appendix A – Concrete Remediation Plan (ordered)

1) Enforce auth on directory-api HTTP routes and add mTLS/TLS to gRPC.
2) Add Alembic migrations execution to auth-api container entrypoint; verify migrations for other services.
3) Lock dependencies; add non-root users and multi-stage minimal images; generate SBOMs and sign images.
4) Add rate limiting/backoff for login and device flows; consider CAPTCHA after N failures.
5) Replace runtime proto generation with build-time codegen for directory gRPC stubs.
6) Harden observability: deny-by-default span attributes, regression tests for PII in metrics/traces.
7) Add service healthchecks in compose and upgrade “depends_on” to health-based.
8) Add integration test suite that spins compose, seeds DB, and runs API flows end-to-end.

Appendix B – Known Non-Prod Flags (confirm off in prod)

- node-agent network_mode: host – dev only.
- directory-api gRPC insecure port – dev only.
- SQLite usage in tests – unit only.

E2E Test Faults (to be appended after execution)

- Attempted to start control-plane services with: docker compose up -d --build postgres redis auth-api directory-api admin-api prometheus grafana jaeger alertmanager

- Failures observed:
  1) auth-api container crash on startup
     - Error: ModuleNotFoundError: No module named 'psycopg2'
     - Root cause: SQLAlchemy URL is set to postgresql://… which uses psycopg2 by default, but the image installs psycopg (v3) via psycopg[binary]. Either install psycopg2(-binary) or change the DSN to postgresql+psycopg://…
     - Impact: auth-api never starts; control-plane unusable.

  2) directory-api container crash on startup
     - Error: ModuleNotFoundError: No module named 'services' when importing services.common.observability during app import.
     - Root cause: The package import assumes a monorepo root on PYTHONPATH. Inside the container, only /app is present, so top-level 'services' is not importable.
     - Impact: directory-api never starts; control-plane unusable.
     - Note: Additionally, gRPC runs in insecure mode (see Critical Findings #1) and would be exposed on 0.0.0.0 when it does start.

  3) admin-api container crash on startup
     - Error: AssertionError: Status code 204 must not have a response body (FastAPI route registration for DELETE /operators/{operator_id}).
     - Root cause: Route declared with 204 but FastAPI detects a response body (response_model or default return). Needs explicit no-body semantics.
     - Impact: admin-api never starts.

  4) node-agent image build fails (when included)
     - Error: Dockerfile parse error: COPY cannot accept a heredoc as a destination (COPY --chmod=755 /dev/stdin … << 'EOF').
     - Root cause: Dockerfile syntax not supported by current builder.
     - Impact: node-agent cannot be built in this environment.

- Net result: The stack is not runnable in its current state; deployment readiness: FAILED.


