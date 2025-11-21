# Operations Documentation

This section covers operational procedures, monitoring, alerting, and troubleshooting for the VPN system in production environments.

---

## Operations Overview

The VPN operations framework focuses on:
- **Proactive Monitoring**: Early detection of issues before user impact
- **Automated Response**: Self-healing systems and automated remediation
- **Incident Management**: Structured response to service disruptions
- **Performance Optimization**: Continuous improvement of system performance
- **Compliance Monitoring**: Ensuring adherence to privacy and security requirements

---

## Service Level Objectives (SLOs)

### Control Plane SLOs

| Service | Availability | Latency (95th) | Error Rate |
|---------|-------------|----------------|------------|
| auth-api | 99.9% | < 200ms | < 0.1% |
| directory-api | 99.9% | < 300ms | < 0.1% |
| admin-api | 99.5% | < 500ms | < 0.5% |

### Data Plane SLOs

| Metric | Target | Measurement |
|--------|--------|-------------|
| Node Availability | 99.5% | Nodes responding to health checks |
| Connection Success Rate | 99% | Successful WireGuard handshakes |
| Tunnel Latency | < 50ms overhead | Client-to-node RTT increase |
| Throughput | > 100 Mbps | Per-node sustained bandwidth |

### User Experience SLOs

| Flow | Success Rate | Latency |
|------|-------------|---------|
| User Registration | 99.5% | < 2s |
| Device Authentication | 99.9% | < 1s |
| VPN Connection | 99% | < 5s |
| Region Selection | 99.9% | < 500ms |

---

## Monitoring and Alerting

### Metrics Collection

#### Application Metrics

**FastAPI Services** (Prometheus format):
```python
# Custom metrics in services
from prometheus_client import Counter, Histogram, Gauge

# Request metrics
http_requests_total = Counter(
    'http_requests_total',
    'Total HTTP requests',
    ['method', 'endpoint', 'status']
)

http_request_duration_seconds = Histogram(
    'http_request_duration_seconds',
    'HTTP request duration',
    ['method', 'endpoint']
)

# Business metrics
active_users_gauge = Gauge(
    'active_users_total',
    'Number of active users'
)

vpn_connections_gauge = Gauge(
    'vpn_connections_active',
    'Active VPN connections',
    ['region', 'node']
)
```

**Node Agent Metrics** (Rust):
```rust
// Node agent metrics
use prometheus::{Counter, Gauge, Histogram, Registry};

pub struct NodeMetrics {
    pub connections_total: Counter,
    pub bandwidth_bytes: Counter,
    pub latency_seconds: Histogram,
    pub cpu_usage: Gauge,
    pub memory_usage: Gauge,
}
```

#### Infrastructure Metrics

**System Metrics**:
- CPU utilization, memory usage, disk I/O
- Network throughput, packet loss, latency
- Container resource consumption
- Database connection pools, query performance

**Kubernetes Metrics**:
- Pod restarts, resource requests/limits
- Node capacity, cluster autoscaling events
- Ingress traffic, service mesh metrics

### Alerting Rules

#### Critical Alerts

```yaml
# prometheus/rules/critical.yml
groups:
  - name: critical
    rules:
    - alert: ServiceDown
      expr: up{job=~"auth-api|directory-api"} == 0
      for: 30s
      labels:
        severity: critical
        team: platform
      annotations:
        summary: "{{ $labels.job }} service is down"
        description: "Service {{ $labels.job }} has been down for more than 30 seconds"
        runbook: "https://docs.vpn.example.com/runbooks/service-down"
        
    - alert: HighErrorRate
      expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.05
      for: 2m
      labels:
        severity: critical
      annotations:
        summary: "High error rate on {{ $labels.job }}"
        description: "Error rate is {{ $value | humanizePercentage }} over 5 minutes"
        
    - alert: DatabaseConnectionFailure
      expr: postgresql_up == 0
      for: 1m
      labels:
        severity: critical
      annotations:
        summary: "Database connection failure"
        description: "Cannot connect to PostgreSQL database"
```

#### Warning Alerts

```yaml
# prometheus/rules/warning.yml
groups:
  - name: warning
    rules:
    - alert: HighLatency
      expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 0.5
      for: 5m
      labels:
        severity: warning
      annotations:
        summary: "High latency on {{ $labels.job }}"
        description: "95th percentile latency is {{ $value }}s"
        
    - alert: NodeAgentUnhealthy
      expr: up{job="node-agents"} == 0
      for: 2m
      labels:
        severity: warning
      annotations:
        summary: "Node agent {{ $labels.instance }} is unhealthy"
        
    - alert: LowDiskSpace
      expr: (node_filesystem_avail_bytes / node_filesystem_size_bytes) < 0.1
      for: 5m
      labels:
        severity: warning
      annotations:
        summary: "Low disk space on {{ $labels.instance }}"
```

### Notification Channels

#### Alertmanager Configuration

```yaml
# alertmanager/alertmanager.yml
global:
  smtp_smarthost: 'smtp.example.com:587'
  smtp_from: 'alerts@vpn.example.com'

route:
  group_by: ['alertname', 'cluster', 'service']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'default'
  routes:
  - match:
      severity: critical
    receiver: 'critical-alerts'
  - match:
      severity: warning
    receiver: 'warning-alerts'

receivers:
- name: 'default'
  email_configs:
  - to: 'ops-team@vpn.example.com'
    subject: 'VPN Alert: {{ .GroupLabels.alertname }}'
    
- name: 'critical-alerts'
  email_configs:
  - to: 'oncall@vpn.example.com'
    subject: 'CRITICAL: {{ .GroupLabels.alertname }}'
  slack_configs:
  - api_url: 'https://hooks.slack.com/services/...'
    channel: '#critical-alerts'
    title: 'Critical Alert'
    
- name: 'warning-alerts'
  slack_configs:
  - api_url: 'https://hooks.slack.com/services/...'
    channel: '#ops-alerts'
    title: 'Warning Alert'
```

---

## Dashboards

### Grafana Dashboard Configuration

#### Service Overview Dashboard

```json
{
  "dashboard": {
    "id": null,
    "title": "VPN Service Overview",
    "tags": ["vpn", "overview"],
    "timezone": "UTC",
    "panels": [
      {
        "id": 1,
        "title": "Request Rate",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total[5m]))",
            "legendFormat": "Requests/sec"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "reqps",
            "thresholds": {
              "steps": [
                {"color": "green", "value": null},
                {"color": "yellow", "value": 100},
                {"color": "red", "value": 1000}
              ]
            }
          }
        }
      },
      {
        "id": 2,
        "title": "Error Rate",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m]))",
            "legendFormat": "Error Rate"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percentunit",
            "max": 1,
            "thresholds": {
              "steps": [
                {"color": "green", "value": null},
                {"color": "yellow", "value": 0.01},
                {"color": "red", "value": 0.05}
              ]
            }
          }
        }
      },
      {
        "id": 3,
        "title": "Response Time",
        "type": "timeseries",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "50th percentile"
          },
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          },
          {
            "expr": "histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "99th percentile"
          }
        ]
      }
    ]
  }
}
```

#### Node Fleet Dashboard

```json
{
  "dashboard": {
    "title": "VPN Node Fleet",
    "panels": [
      {
        "title": "Node Status",
        "type": "stat",
        "targets": [
          {
            "expr": "count(up{job=\"node-agents\"} == 1)",
            "legendFormat": "Online Nodes"
          },
          {
            "expr": "count(up{job=\"node-agents\"} == 0)",
            "legendFormat": "Offline Nodes"
          }
        ]
      },
      {
        "title": "Active Connections by Region",
        "type": "piechart",
        "targets": [
          {
            "expr": "sum by (region) (vpn_connections_active)",
            "legendFormat": "{{ region }}"
          }
        ]
      },
      {
        "title": "Bandwidth Usage",
        "type": "timeseries",
        "targets": [
          {
            "expr": "sum by (node) (rate(node_network_transmit_bytes_total[5m]))",
            "legendFormat": "{{ node }} TX"
          },
          {
            "expr": "sum by (node) (rate(node_network_receive_bytes_total[5m]))",
            "legendFormat": "{{ node }} RX"
          }
        ]
      }
    ]
  }
}
```

### Custom Dashboards

#### User Experience Dashboard

Tracks user-facing metrics:
- Registration success rate
- Login latency
- Connection establishment time
- Regional performance comparison

#### Security Dashboard

Monitors security-related events:
- Failed authentication attempts
- Certificate expiration warnings
- Unusual traffic patterns
- GDPR request processing

---

## Runbooks

### Service Down Runbook

**Alert**: `ServiceDown`
**Severity**: Critical
**Response Time**: < 5 minutes

#### Investigation Steps

1. **Check Service Health**:
   ```bash
   # Verify service status
   kubectl get pods -n vpn-system -l app=auth-api
   kubectl describe pod <pod-name> -n vpn-system
   
   # Check recent logs
   kubectl logs -f <pod-name> -n vpn-system --tail=100
   ```

2. **Check Dependencies**:
   ```bash
   # Database connectivity
   kubectl exec -it <auth-api-pod> -- curl -f $DATABASE_URL
   
   # Redis connectivity  
   kubectl exec -it <auth-api-pod> -- redis-cli -u $REDIS_URL ping
   ```

3. **Check Resource Usage**:
   ```bash
   # CPU and memory usage
   kubectl top pods -n vpn-system
   
   # Node resources
   kubectl top nodes
   ```

#### Resolution Steps

1. **Restart Service**:
   ```bash
   kubectl rollout restart deployment/auth-api -n vpn-system
   kubectl rollout status deployment/auth-api -n vpn-system
   ```

2. **Scale Up if Resource Constrained**:
   ```bash
   kubectl scale deployment/auth-api --replicas=5 -n vpn-system
   ```

3. **Emergency Rollback**:
   ```bash
   kubectl rollout undo deployment/auth-api -n vpn-system
   ```

### High Latency Runbook

**Alert**: `HighLatency`
**Severity**: Warning
**Response Time**: < 15 minutes

#### Investigation Steps

1. **Identify Slow Endpoints**:
   ```bash
   # Query Prometheus for slow endpoints
   curl -G 'http://prometheus:9090/api/v1/query' \
     --data-urlencode 'query=topk(5, histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])))'
   ```

2. **Check Database Performance**:
   ```bash
   # Active queries
   kubectl exec -it postgres-pod -- psql -c "SELECT * FROM pg_stat_activity WHERE state = 'active';"
   
   # Slow queries
   kubectl exec -it postgres-pod -- psql -c "SELECT * FROM pg_stat_statements ORDER BY mean_time DESC LIMIT 10;"
   ```

3. **Check Resource Utilization**:
   ```bash
   # Service resource usage
   kubectl top pods -n vpn-system
   
   # Database resource usage
   kubectl top pods -n database
   ```

#### Resolution Steps

1. **Scale Services**:
   ```bash
   kubectl scale deployment/auth-api --replicas=5 -n vpn-system
   ```

2. **Optimize Database**:
   ```bash
   # Analyze and vacuum tables
   kubectl exec -it postgres-pod -- psql -c "ANALYZE; VACUUM;"
   ```

3. **Enable Caching**:
   ```bash
   # Increase Redis memory if needed
   kubectl patch deployment redis --patch '{"spec":{"template":{"spec":{"containers":[{"name":"redis","resources":{"limits":{"memory":"2Gi"}}}]}}}}'
   ```

### Node Agent Failure Runbook

**Alert**: `NodeAgentUnhealthy`
**Severity**: Warning
**Response Time**: < 10 minutes

#### Investigation Steps

1. **Check Node Status**:
   ```bash
   # Node agent logs
   docker logs vpn-node-agent --tail=100
   
   # WireGuard interface status
   docker exec vpn-node-agent wg show
   ```

2. **Check Network Connectivity**:
   ```bash
   # Test control plane connectivity
   docker exec vpn-node-agent curl -f $DIRECTORY_HTTP_ADDR/healthz
   
   # Test gRPC connectivity
   docker exec vpn-node-agent grpcurl -plaintext $DIRECTORY_GRPC_ADDR list
   ```

3. **Check System Resources**:
   ```bash
   # System resources on node host
   top
   df -h
   netstat -tuln | grep 51820
   ```

#### Resolution Steps

1. **Restart Node Agent**:
   ```bash
   docker restart vpn-node-agent
   ```

2. **Regenerate Certificates** (if mTLS issues):
   ```bash
   # Regenerate node certificate
   ./tools/pki/issue_node_cert.sh $(hostname)
   
   # Restart with new certificate
   docker restart vpn-node-agent
   ```

3. **Replace Node** (if hardware issues):
   ```bash
   # Drain connections from failing node
   curl -X PUT $DIRECTORY_HTTP_ADDR/nodes/$NODE_ID \
     -H "Content-Type: application/json" \
     -d '{"status": "maintenance"}'
   
   # Provision replacement node
   terraform apply -var="node_count=+1"
   ```

---

## Incident Response

### Incident Classification

#### Severity Levels

**P0 - Critical**:
- Complete service outage
- Data breach or security incident
- Compliance violation

**P1 - High**:
- Partial service degradation
- Single region outage
- Performance significantly impacted

**P2 - Medium**:
- Minor feature issues
- Single node failures
- Non-critical alerts

**P3 - Low**:
- Cosmetic issues
- Documentation updates
- Planned maintenance

### Incident Response Process

#### P0/P1 Incident Response

1. **Detection** (0-5 minutes):
   - Automated alerting triggers
   - On-call engineer notified
   - Incident commander assigned

2. **Assessment** (5-15 minutes):
   - Determine impact and scope
   - Classify incident severity
   - Assemble response team

3. **Response** (15-60 minutes):
   - Implement immediate mitigation
   - Communicate with stakeholders
   - Document actions taken

4. **Resolution** (varies):
   - Apply permanent fix
   - Verify service restoration
   - Monitor for recurrence

5. **Post-Incident** (24-48 hours):
   - Conduct post-mortem
   - Document lessons learned
   - Implement preventive measures

#### Communication Plan

**Internal Communication**:
- Slack: `#incident-response` channel
- Email: incident-team@vpn.example.com
- Phone: On-call rotation

**External Communication**:
- Status page: status.vpn.example.com
- Customer notifications via email
- Social media updates if needed

### Post-Incident Review

#### Post-Mortem Template

```markdown
# Incident Post-Mortem: [YYYY-MM-DD] [Brief Description]

## Summary
- **Date/Time**: 
- **Duration**: 
- **Impact**: 
- **Root Cause**: 

## Timeline
- **HH:MM** - Initial detection
- **HH:MM** - Response team assembled
- **HH:MM** - Mitigation implemented
- **HH:MM** - Service restored

## What Went Well
- 

## What Could Be Improved
- 

## Action Items
- [ ] Action item 1 (Owner: @person, Due: YYYY-MM-DD)
- [ ] Action item 2 (Owner: @person, Due: YYYY-MM-DD)

## Lessons Learned
- 
```

---

## Capacity Planning

### Resource Monitoring

#### Capacity Metrics

**Compute Resources**:
```promql
# CPU utilization trend
avg_over_time(cpu_usage_percent[7d])

# Memory usage growth
increase(memory_usage_bytes[30d])

# Network bandwidth utilization
rate(network_bytes_total[5m])
```

**Storage Growth**:
```promql
# Database size growth
increase(postgresql_database_size_bytes[30d])

# Log volume growth
increase(log_volume_bytes[7d])
```

#### Scaling Triggers

**Horizontal Pod Autoscaler**:
```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: auth-api-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: auth-api
  minReplicas: 2
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

**Cluster Autoscaler**:
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: cluster-autoscaler-status
data:
  nodes.max: "20"
  scale-down-delay-after-add: "10m"
  scale-down-unneeded-time: "10m"
```

### Growth Projections

#### User Growth Planning

| Metric | Current | 3 Months | 6 Months | 12 Months |
|--------|---------|----------|----------|-----------|
| Active Users | 1,000 | 5,000 | 15,000 | 50,000 |
| Concurrent Connections | 500 | 2,500 | 7,500 | 25,000 |
| API Requests/sec | 100 | 500 | 1,500 | 5,000 |
| Data Transfer/day | 1TB | 5TB | 15TB | 50TB |

#### Infrastructure Scaling Plan

**Phase 1** (0-5K users):
- 3 control plane nodes
- 5 VPN nodes across 3 regions
- Single database instance

**Phase 2** (5K-15K users):
- 5 control plane nodes
- 15 VPN nodes across 5 regions
- Database read replicas

**Phase 3** (15K+ users):
- Auto-scaling control plane
- 50+ VPN nodes globally
- Database sharding/clustering

---

## Maintenance Procedures

### Scheduled Maintenance

#### Maintenance Windows

**Regular Maintenance**:
- **Weekly**: Security updates, minor patches
- **Monthly**: Database maintenance, certificate rotation
- **Quarterly**: Major version updates, infrastructure changes

**Maintenance Schedule**:
- **Primary Window**: Sundays 02:00-06:00 UTC
- **Emergency Window**: Any time with 4-hour notice
- **Blackout Periods**: Major holidays, high-traffic events

#### Pre-Maintenance Checklist

```markdown
- [ ] Maintenance window scheduled and communicated
- [ ] Backup verification completed
- [ ] Rollback plan documented and tested
- [ ] Monitoring alerts adjusted for maintenance
- [ ] Customer notification sent (if user-impacting)
- [ ] On-call engineer assigned
- [ ] Change approval obtained
```

### Database Maintenance

#### Regular Database Tasks

```bash
#!/bin/bash
# scripts/db_maintenance.sh

# Analyze table statistics
psql $DATABASE_URL -c "ANALYZE;"

# Vacuum to reclaim space
psql $DATABASE_URL -c "VACUUM (ANALYZE, VERBOSE);"

# Reindex if needed
psql $DATABASE_URL -c "REINDEX DATABASE vpn_db;"

# Update table statistics
psql $DATABASE_URL -c "SELECT schemaname, tablename, last_vacuum, last_autovacuum FROM pg_stat_user_tables;"
```

#### Certificate Rotation

```bash
#!/bin/bash
# scripts/rotate_certificates.sh

# Rotate JWT signing keys
kubectl create secret generic jwt-signing-keys-new \
  --from-file=private_key=new_private_key.pem \
  --from-file=public_key=new_public_key.pem

# Update deployment to use new keys
kubectl patch deployment auth-api \
  --patch '{"spec":{"template":{"spec":{"volumes":[{"name":"jwt-keys","secret":{"secretName":"jwt-signing-keys-new"}}]}}}}'

# Wait for rollout
kubectl rollout status deployment/auth-api

# Clean up old keys
kubectl delete secret jwt-signing-keys-old
```

---

## Security Operations

### Security Monitoring

#### Security Metrics

```promql
# Failed authentication attempts
rate(auth_attempts_total{status="failed"}[5m])

# Suspicious IP addresses
topk(10, count by (source_ip) (auth_attempts_total{status="failed"}))

# Certificate expiration warnings
(cert_expiry_timestamp - time()) / 86400 < 30
```

#### Security Alerts

```yaml
# security-alerts.yml
groups:
  - name: security
    rules:
    - alert: HighFailedAuthRate
      expr: rate(auth_attempts_total{status="failed"}[5m]) > 10
      for: 2m
      labels:
        severity: warning
      annotations:
        summary: "High failed authentication rate"
        
    - alert: CertificateExpiringSoon
      expr: (cert_expiry_timestamp - time()) / 86400 < 7
      labels:
        severity: critical
      annotations:
        summary: "Certificate expiring in {{ $value }} days"
```

### Compliance Monitoring

#### GDPR Compliance Checks

```bash
#!/bin/bash
# scripts/gdpr_compliance_check.sh

# Check for PII in logs
grep -r "email\|password\|ip_address" /var/log/vpn/ && echo "ALERT: PII found in logs"

# Verify data retention policies
psql $DATABASE_URL -c "SELECT COUNT(*) FROM users WHERE created_at < NOW() - INTERVAL '2 years';"

# Check consent records
psql $DATABASE_URL -c "SELECT COUNT(*) FROM users WHERE consent IS NULL OR consent = false;"
```

#### Audit Logging

```python
# Audit log format
import json
import logging

audit_logger = logging.getLogger('audit')

def log_audit_event(event_type, user_id, details):
    audit_event = {
        'timestamp': datetime.utcnow().isoformat(),
        'event_type': event_type,
        'user_id': user_id,
        'details': details,
        'source_ip': request.remote_addr,
        'user_agent': request.headers.get('User-Agent')
    }
    audit_logger.info(json.dumps(audit_event))
```