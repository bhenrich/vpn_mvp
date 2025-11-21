# Runbook: Instance Down

Severity: page

When to trigger: Alert InstanceDown fires (target up == 0 for >2m).

Immediate actions:
- Confirm container state:
  - docker compose ps
  - docker compose logs <service>
- If crashloop:
  - Capture last 200 lines of logs.
  - Check recent config or secret changes.
- Check node resource pressure (disk full, OOM, CPU throttling).

Mitigation:
- Restart the service:
  - docker compose restart <service>
- If repeated failures:
  - Roll back to last known good image.
  - Disable problematic feature flags in admin-api if applicable.

Post-incident:
- Open ticket with root cause and remediation plan.
- Add health checks and readiness gates to prevent serving traffic when unhealthy.


