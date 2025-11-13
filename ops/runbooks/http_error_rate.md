# Runbook: High HTTP Error Rate

Severity: page

When to trigger: Alert HighErrorRate5m fires (5xx/4xx ratio > 2% for 10m).

Immediate actions:
- Check service health endpoints:
  - auth-api: GET http://auth-api:8080/healthz
  - directory-api: GET http://directory-api:8081/healthz
  - admin-api: GET http://admin-api:8082/healthz
- Inspect recent deploys/config changes. Roll back if necessary.
- Check dependency health:
  - Postgres reachable and healthy
  - Redis reachable and healthy
- Examine logs for recurring exceptions (but ensure no PII is included).

Deep dive:
- Use Jaeger to locate failing spans; identify hot endpoints and error sources.
- Validate tokens/JWKS rotation if auth-related errors present.
- For database-related errors: verify migrations, locks, connection pool saturation.
- For rate-limiting or policy errors: confirm Redis counters and TTL logic.

Mitigation:
- Roll back last deployment or flag off feature toggles via admin-api if applicable.
- Increase resources if sustained saturation (CPU/memory) is observed, then plan a capacity follow-up.
- If dependency degraded (DB/Redis), fail over or restart.

Post-incident:
- Create a ticket with timeline and root cause.
- Add or adjust alerts and tests to prevent recurrence.


