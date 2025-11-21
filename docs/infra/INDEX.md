# Infrastructure Documentation

This section covers infrastructure setup, deployment patterns, and operational procedures for the VPN system across development, staging, and production environments.

---

## Infrastructure Overview

The VPN infrastructure is designed for:
- **Scalability**: Horizontal scaling of services and nodes
- **Reliability**: Multi-region deployment with failover capabilities  
- **Security**: Defense in depth with network segmentation and encryption
- **Compliance**: Data residency and privacy controls
- **Observability**: Comprehensive monitoring and alerting

---

## Deployment Environments

### Development Environment

**Purpose**: Local development and testing
**Infrastructure**: Single-host Docker Compose stack

#### Quick Start

```bash
# Clone repository
git clone https://github.com/example/vpn-mvp.git
cd vpn-mvp

# Install dependencies
make bootstrap

# Start full stack
docker compose up --build

# Verify services
make health-check
```

#### Services Included

```yaml
# docker-compose.yml (simplified)
services:
  postgres:
    image: postgres:15
    ports: ["5432:5432"]
    
  redis:
    image: redis:7-alpine
    ports: ["6379:6379"]
    
  auth-api:
    build: ./services/auth-api
    ports: ["8080:8080"]
    depends_on: [postgres, redis]
    
  directory-api:
    build: ./services/directory-api
    ports: ["8081:8081", "50051:50051"]
    depends_on: [postgres, redis]
    
  admin-api:
    build: ./services/admin-api
    ports: ["8082:8082"]
    depends_on: [postgres, redis]
    
  node-agent-de-berlin:
    build: ./nodes/agent
    environment:
      - NODE_REGION_CODE=DE
      - NODE_REGION_CITY=Berlin
      - PUBLIC_ENDPOINT=de-berlin.dev.local
    privileged: true  # Required for WireGuard
    
  prometheus:
    image: prom/prometheus:latest
    ports: ["9090:9090"]
    
  grafana:
    image: grafana/grafana:latest
    ports: ["3000:3000"]
```

#### Environment Configuration

Each service uses environment files:

```bash
# services/auth-api/.env
DATABASE_URL=postgresql+asyncpg://postgres:example@postgres:5432/postgres
REDIS_URL=redis://redis:6379/0
JWT_SECRET_KEY=dev-secret-key-change-in-production
JWT_ALGORITHM=RS256
JWT_ACCESS_TOKEN_EXPIRE_MINUTES=15
JWT_REFRESH_TOKEN_EXPIRE_DAYS=7

# services/directory-api/.env  
DATABASE_URL=postgresql+asyncpg://postgres:example@postgres:5432/postgres
REDIS_URL=redis://redis:6379/1
GRPC_PORT=50051
```

### Staging Environment

**Purpose**: Pre-production testing and validation
**Infrastructure**: Multi-container deployment with external dependencies

#### Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Load Balancer │    │   Control Plane │    │   Node Fleet    │
│                 │    │                 │    │                 │
│ • Nginx/HAProxy │    │ • auth-api      │    │ • EU Nodes      │
│ • SSL Termination│───►│ • directory-api │◄───│ • US Nodes      │
│ • Rate Limiting │    │ • admin-api     │    │ • APAC Nodes    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │   Data Layer    │
                       │                 │
                       │ • PostgreSQL    │
                       │ • Redis Cluster │
                       │ • Monitoring    │
                       └─────────────────┘
```

#### Deployment with Docker Swarm

```yaml
# docker-compose.staging.yml
version: '3.8'

services:
  auth-api:
    image: vpn/auth-api:${VERSION}
    deploy:
      replicas: 2
      update_config:
        parallelism: 1
        delay: 10s
      restart_policy:
        condition: on-failure
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
    networks:
      - control-plane
      
  directory-api:
    image: vpn/directory-api:${VERSION}
    deploy:
      replicas: 2
    networks:
      - control-plane
      - node-mesh
      
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/ssl/certs
    networks:
      - control-plane

networks:
  control-plane:
    driver: overlay
  node-mesh:
    driver: overlay
```

### Production Environment

**Purpose**: Live customer traffic
**Infrastructure**: Kubernetes or managed cloud services

#### Cloud Architecture (AWS Example)

```
┌────────────────────────────────────────────────────────────┐
│                        AWS VPC                             │
│                                                            │
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │   Public Subnet │    │  Private Subnet │                │
│  │                 │    │                 │                │
│  │ • ALB           │    │ • EKS Cluster   │                │
│  │ • NAT Gateway   │────┤ • Control Plane │                │
│  │ • Bastion       │    │ • Worker Nodes  │                │
│  └─────────────────┘    └─────────────────┘                │
│                                  │                         │
│                         ┌─────────────────┐                │
│                         │  Data Subnet    │                │
│                         │                 │                │
│                         │ • RDS Postgres  │                │
│                         │ • ElastiCache   │                │
│                         └─────────────────┘                │
└────────────────────────────────────────────────────────────┘
```

#### Kubernetes Deployment

```yaml
# k8s/auth-api-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: auth-api
  namespace: vpn-system
spec:
  replicas: 3
  selector:
    matchLabels:
      app: auth-api
  template:
    metadata:
      labels:
        app: auth-api
    spec:
      containers:
      - name: auth-api
        image: vpn/auth-api:v1.0.0
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: database-credentials
              key: url
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /healthz
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
```

---

## Container Images

### Build System

All services use multi-stage Docker builds for optimization:

```dockerfile
# services/auth-api/Dockerfile
FROM python:3.11-slim as builder

WORKDIR /app
COPY requirements.txt .
RUN pip install --user --no-cache-dir -r requirements.txt

FROM python:3.11-slim as runtime

# Create non-root user
RUN useradd --create-home --shell /bin/bash app

# Copy dependencies
COPY --from=builder /root/.local /home/app/.local
ENV PATH=/home/app/.local/bin:$PATH

# Copy application
WORKDIR /app
COPY --chown=app:app . .

USER app
EXPOSE 8080

CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8080"]
```

### Image Registry

**Development**: Local Docker registry
**Production**: AWS ECR / Google Container Registry / Azure ACR

```bash
# Build and push images
make build-images
make push-images VERSION=v1.0.0

# Security scanning
make scan-images
```

### Image Security

- **Base Images**: Official distroless or Alpine images
- **Vulnerability Scanning**: Integrated with CI/CD pipeline
- **SBOM Generation**: Software Bill of Materials for all images
- **Signature Verification**: Cosign signatures for release images

---

## Database Infrastructure

### PostgreSQL Setup

#### Development (Docker)

```yaml
postgres:
  image: postgres:15
  environment:
    POSTGRES_DB: vpn_db
    POSTGRES_USER: postgres
    POSTGRES_PASSWORD: example
  volumes:
    - postgres_data:/var/lib/postgresql/data
    - ./migrations:/docker-entrypoint-initdb.d
  ports:
    - "5432:5432"
```

#### Production (Managed Service)

**AWS RDS Configuration**:
```json
{
  "DBInstanceClass": "db.r6g.large",
  "Engine": "postgres",
  "EngineVersion": "15.4",
  "MultiAZ": true,
  "StorageType": "gp3",
  "AllocatedStorage": 100,
  "StorageEncrypted": true,
  "BackupRetentionPeriod": 7,
  "DeletionProtection": true,
  "VpcSecurityGroupIds": ["sg-control-plane"],
  "DBSubnetGroupName": "vpn-db-subnet-group"
}
```

#### Schema Migrations

Using Alembic for database migrations:

```bash
# Generate migration
cd services/auth-api
alembic revision --autogenerate -m "Add user consent fields"

# Apply migrations
alembic upgrade head

# Rollback if needed
alembic downgrade -1
```

### Redis Configuration

#### Cluster Setup (Production)

```yaml
# redis-cluster.yml
apiVersion: v1
kind: ConfigMap
metadata:
  name: redis-config
data:
  redis.conf: |
    cluster-enabled yes
    cluster-config-file nodes.conf
    cluster-node-timeout 5000
    appendonly yes
    protected-mode no
    bind 0.0.0.0
    port 6379
```

#### Persistence and Backup

- **AOF**: Append-only file for durability
- **RDB**: Point-in-time snapshots
- **Backup Strategy**: Daily snapshots to S3/GCS

---

## Network Infrastructure

### Service Mesh (Production)

Using Istio for service-to-service communication:

```yaml
# istio-config.yaml
apiVersion: install.istio.io/v1alpha1
kind: IstioOperator
metadata:
  name: control-plane
spec:
  values:
    global:
      meshID: vpn-mesh
      network: vpn-network
  components:
    pilot:
      k8s:
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
```

### Load Balancing

#### Application Load Balancer (AWS)

```yaml
# alb-ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: vpn-api-ingress
  annotations:
    kubernetes.io/ingress.class: alb
    alb.ingress.kubernetes.io/scheme: internet-facing
    alb.ingress.kubernetes.io/target-type: ip
    alb.ingress.kubernetes.io/ssl-policy: ELBSecurityPolicy-TLS-1-2-2017-01
spec:
  tls:
  - hosts:
    - auth.vpn.example.com
    - directory.vpn.example.com
  rules:
  - host: auth.vpn.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: auth-api
            port:
              number: 8080
```

### DNS and CDN

**Route 53 Configuration**:
```json
{
  "Name": "auth.vpn.example.com",
  "Type": "A",
  "AliasTarget": {
    "DNSName": "alb-123456789.us-west-2.elb.amazonaws.com",
    "EvaluateTargetHealth": true
  }
}
```

**CloudFront Distribution**:
- Static assets (client downloads, updates)
- Global edge locations for low latency
- WAF integration for DDoS protection

---

## Node Infrastructure

### Node Agent Deployment

#### Bare Metal / VPS

```bash
# Install Docker
curl -fsSL https://get.docker.com | sh

# Deploy node agent
docker run -d \
  --name vpn-node \
  --restart unless-stopped \
  --privileged \
  --network host \
  -e NODE_REGION_CODE=US \
  -e NODE_REGION_CITY="New York" \
  -e PUBLIC_ENDPOINT=us-ny.vpn.example.com:51820 \
  -e DIRECTORY_HTTP_ADDR=https://directory.vpn.example.com \
  -e DIRECTORY_GRPC_ADDR=directory.vpn.example.com:443 \
  vpn/node-agent:latest
```

#### Kubernetes DaemonSet

```yaml
# node-agent-daemonset.yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: vpn-node-agent
  namespace: vpn-nodes
spec:
  selector:
    matchLabels:
      app: vpn-node-agent
  template:
    metadata:
      labels:
        app: vpn-node-agent
    spec:
      hostNetwork: true
      containers:
      - name: node-agent
        image: vpn/node-agent:latest
        securityContext:
          privileged: true
        env:
        - name: NODE_REGION_CODE
          valueFrom:
            fieldRef:
              fieldPath: metadata.labels['topology.kubernetes.io/region']
        - name: PUBLIC_ENDPOINT
          valueFrom:
            fieldRef:
              fieldPath: status.hostIP
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 256Mi
```

### Geographic Distribution

#### Regional Deployment Strategy

```
Americas:
├── us-east-1 (Virginia)
├── us-west-2 (Oregon)  
├── ca-central-1 (Canada)
└── sa-east-1 (Brazil)

Europe:
├── eu-west-1 (Ireland)
├── eu-central-1 (Frankfurt)
├── eu-north-1 (Stockholm)
└── eu-south-1 (Milan)

Asia-Pacific:
├── ap-northeast-1 (Tokyo)
├── ap-southeast-1 (Singapore)
├── ap-south-1 (Mumbai)
└── ap-southeast-2 (Sydney)
```

#### Node Capacity Planning

| Region | Expected Load | Node Count | Instance Type |
|--------|---------------|------------|---------------|
| US-East | 10,000 users | 5 nodes | c5n.large |
| EU-West | 8,000 users | 4 nodes | c5n.large |
| AP-Southeast | 5,000 users | 3 nodes | c5n.medium |

---

## Security Infrastructure

### Certificate Management

#### PKI Hierarchy

```
Root CA (Offline)
├── Intermediate CA (Online, 90d)
    ├── Service Certificates (30d)
    │   ├── auth-api.vpn.example.com
    │   ├── directory-api.vpn.example.com
    │   └── admin-api.vpn.example.com
    └── Node Certificates (7d)
        ├── us-east-node-001.vpn.example.com
        ├── eu-west-node-001.vpn.example.com
        └── ...
```

#### Automated Certificate Rotation

```bash
#!/bin/bash
# tools/pki/rotate_certificates.sh

# Rotate intermediate CA (every 90 days)
./rotate_intermediate.sh

# Rotate service certificates (every 30 days)
for service in auth-api directory-api admin-api; do
    ./issue_service_cert.sh $service
done

# Rotate node certificates (every 7 days)
kubectl get nodes -l role=vpn-node -o name | while read node; do
    ./issue_node_cert.sh $node
done
```

### Secrets Management

#### Kubernetes Secrets

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: database-credentials
  namespace: vpn-system
type: Opaque
data:
  url: cG9zdGdyZXNxbCsuLi4=  # base64 encoded
  
---
apiVersion: v1
kind: Secret
metadata:
  name: jwt-signing-keys
  namespace: vpn-system
type: Opaque
data:
  private_key: LS0tLS1CRUdJTi4uLg==  # base64 encoded RSA private key
  public_key: LS0tLS1CRUdJTi4uLg==   # base64 encoded RSA public key
```

#### External Secrets Operator

```yaml
# external-secret.yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: database-credentials
spec:
  refreshInterval: 1h
  secretStoreRef:
    name: aws-secrets-manager
    kind: SecretStore
  target:
    name: database-credentials
  data:
  - secretKey: url
    remoteRef:
      key: vpn/database
      property: connection_string
```

---

## Monitoring Infrastructure

### Prometheus Setup

#### Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "rules/*.yml"

scrape_configs:
  - job_name: 'auth-api'
    static_configs:
      - targets: ['auth-api:8080']
    metrics_path: /metrics
    
  - job_name: 'directory-api'
    static_configs:
      - targets: ['directory-api:8081']
      
  - job_name: 'node-agents'
    kubernetes_sd_configs:
      - role: pod
        namespaces:
          names: ['vpn-nodes']
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_label_app]
        action: keep
        regex: vpn-node-agent
```

#### Alerting Rules

```yaml
# rules/vpn-alerts.yml
groups:
  - name: vpn.rules
    rules:
    - alert: HighErrorRate
      expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
      for: 5m
      labels:
        severity: warning
      annotations:
        summary: "High error rate detected"
        
    - alert: NodeAgentDown
      expr: up{job="node-agents"} == 0
      for: 1m
      labels:
        severity: critical
      annotations:
        summary: "Node agent is down"
```

### Grafana Dashboards

#### Service Health Dashboard

```json
{
  "dashboard": {
    "title": "VPN Service Health",
    "panels": [
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(http_requests_total[5m])",
            "legendFormat": "{{service}}"
          }
        ]
      },
      {
        "title": "Response Time",
        "type": "graph", 
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          }
        ]
      }
    ]
  }
}
```

---

## Backup and Disaster Recovery

### Database Backups

#### Automated Backup Script

```bash
#!/bin/bash
# scripts/backup_database.sh

BACKUP_DIR="/backups/$(date +%Y-%m-%d)"
mkdir -p $BACKUP_DIR

# Full database backup
pg_dump $DATABASE_URL > $BACKUP_DIR/full_backup.sql

# Compress and encrypt
gzip $BACKUP_DIR/full_backup.sql
gpg --encrypt --recipient backup@vpn.example.com $BACKUP_DIR/full_backup.sql.gz

# Upload to S3
aws s3 cp $BACKUP_DIR/full_backup.sql.gz.gpg s3://vpn-backups/database/

# Cleanup old backups (keep 30 days)
find /backups -type d -mtime +30 -exec rm -rf {} \;
```

### Infrastructure as Code

#### Terraform Configuration

```hcl
# infrastructure/main.tf
provider "aws" {
  region = var.aws_region
}

module "vpc" {
  source = "./modules/vpc"
  
  cidr_block = "10.0.0.0/16"
  availability_zones = ["us-west-2a", "us-west-2b", "us-west-2c"]
}

module "eks" {
  source = "./modules/eks"
  
  cluster_name = "vpn-cluster"
  vpc_id = module.vpc.vpc_id
  subnet_ids = module.vpc.private_subnet_ids
  
  node_groups = {
    control_plane = {
      instance_types = ["m5.large"]
      min_size = 2
      max_size = 10
      desired_size = 3
    }
    
    vpn_nodes = {
      instance_types = ["c5n.large"]
      min_size = 1
      max_size = 20
      desired_size = 5
    }
  }
}
```

### Disaster Recovery Plan

#### RTO/RPO Targets

| Component | RTO | RPO | Recovery Strategy |
|-----------|-----|-----|-------------------|
| Control Plane | 15 minutes | 5 minutes | Multi-AZ deployment |
| Database | 30 minutes | 1 minute | Read replicas + PITR |
| Node Fleet | 5 minutes | 0 | Stateless, auto-scaling |
| Monitoring | 10 minutes | 5 minutes | Separate cluster |

#### Recovery Procedures

1. **Database Recovery**:
   ```bash
   # Restore from point-in-time backup
   aws rds restore-db-instance-to-point-in-time \
     --source-db-instance-identifier vpn-db-prod \
     --target-db-instance-identifier vpn-db-recovery \
     --restore-time 2024-01-01T12:00:00Z
   ```

2. **Service Recovery**:
   ```bash
   # Deploy to backup region
   kubectl config use-context backup-cluster
   kubectl apply -f k8s/
   
   # Update DNS to point to backup
   aws route53 change-resource-record-sets \
     --hosted-zone-id Z123456789 \
     --change-batch file://failover-dns.json
   ```

---

## Cost Optimization

### Resource Sizing

#### Development Environment
- **Total Cost**: ~$50/month
- **Compute**: 2 vCPU, 4GB RAM
- **Storage**: 20GB SSD
- **Network**: Minimal egress

#### Production Environment
- **Control Plane**: ~$500/month
  - 3x m5.large instances (EKS)
  - RDS db.r6g.large Multi-AZ
  - ElastiCache r6g.large cluster
  
- **Node Fleet**: ~$200/node/month
  - c5n.large instances with enhanced networking
  - 1TB monthly data transfer per node
  - Regional distribution costs

### Cost Monitoring

```yaml
# cost-alerts.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: cost-alerts
data:
  budget.json: |
    {
      "BudgetName": "vpn-infrastructure",
      "BudgetLimit": {
        "Amount": "2000",
        "Unit": "USD"
      },
      "TimeUnit": "MONTHLY",
      "CostFilters": {
        "TagKey": ["Project"],
        "TagValue": ["vpn-mvp"]
      }
    }
```

---

For operational procedures and troubleshooting, see:
- [Operations Guide](../ops/INDEX.md)
- [Security Documentation](../security/)
- [Development Setup](../../CONTRIBUTING.md)
- [Architecture Overview](../architecture/ARCHITECTURE.md)
