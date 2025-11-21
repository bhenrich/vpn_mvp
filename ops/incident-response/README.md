# Incident Response Workflow (Privacy-First)

Roles:
- Incident Commander (IC): coordinates response, assigns owners.
- Communications Lead: status updates to stakeholders.
- Investigators: service owners triaging issues.

Severities:
- SEV-1 (page): outage/data-at-risk; on-call responds immediately.
- SEV-2 (page): degraded core functionality.
- SEV-3 (ticket): minor degradation or latency.

SLOs:
- API availability: 99.9% monthly; P95 latency < 500ms.
- Page response time: IC engaged in < 5 minutes for SEV-1/2.

Runbooks:
- Prometheus alerts link to runbooks in /runbooks inside Prometheus container and in repo at ops/runbooks/.

Process:
1) Triage alert; confirm impact using dashboards and health endpoints.
2) Declare severity; assign IC and Investigators.
3) Mitigate quickly (rollback, scale, toggle flags), then diagnose root cause.
4) Communicate status every 15–30 minutes for SEV-1/2.
5) Post-incident review within 48 hours with action items.

Privacy constraints:
- No PII in logs, traces, or metrics.
- Use aggregate metrics and anonymized identifiers only.


