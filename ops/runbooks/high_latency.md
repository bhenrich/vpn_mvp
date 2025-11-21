# Runbook: High Latency P95

Severity: ticket

When to trigger: Alert HighLatencyP95 fires (P95 > 500ms for 10m).

Immediate actions:
- Check Grafana dashboard Aggregate Overview for spikes (CPU, memory, GC, DB time).
- Verify no background jobs or migrations are running.
- Inspect Jaeger traces:
  - Identify spans with highest self-time.
  - Check external calls (DB, Redis, HTTP) for slowness.

Deep dive:
- Database:
  - Check slow query logs and lock wait events.
  - Review connection pool sizes and saturation.
- Application:
  - Evaluate throughput vs. latency; confirm no synchronous heavy work on request path.
  - Validate thread/event loop starvation (e.g., excessive blocking).
- Network:
  - Inspect container/node network I/O; packet loss or DNS issues.

Mitigation:
- Scale up temporarily; adjust pool sizes cautiously.
- Add caching where appropriate (ensure privacy constraints).
- Optimize hot endpoints and queries; ship fix behind a feature flag if needed.

Post-incident:
- Capture flamegraphs/profiles if reproducible.
- Add SLO budgets and performance regression tests.


