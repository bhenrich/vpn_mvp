# Deployment Guides

This section provides comprehensive deployment guides for all components of the VPN system across different environments and platforms.

---

## Deployment Overview

The VPN system supports multiple deployment patterns:

- **Development**: Single-host Docker Compose for local development
- **Staging**: Multi-container deployment for testing and validation
- **Production**: Scalable cloud deployment with high availability
- **Hybrid**: On-premises control plane with cloud node fleet
- **Edge**: Distributed node deployment across multiple providers

---

## Quick Start Deployment

### Local Development Stack

For rapid development and testing:

```bash
# Clone repository
git clone https://github.com/example/vpn-mvp.git
cd vpn-mvp

# Setup environment
make bootstrap

# Start complete stack
docker compose up --build

# Verify deployment
make health-check
```

This deploys:
- PostgreSQL database with initial schema
- Redis for session storage
- All three API services (auth, directory, admin)
- Single development node agent
- Prometheus and Grafana for monitoring

### Production Quick Deploy

For a minimal production deployment:

```bash
# Set environment variables
export VPN_DOMAIN=vpn.example.com
export DATABASE_URL=postgresql://user:pass@db.example.com/vpn
export REDIS_URL=redis://cache.example.com:6379

# Deploy control plane
docker compose -f docker-compose.prod.yml up -d

# Deploy node agents
./scripts/deploy_nodes.sh --region us-east-1 --count 3
```

---

## Control Plane Deployment

### Docker Compose Deployment

#### Production Configuration

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  auth-api:
    image: vpn/auth-api:${VERSION:-latest}
    restart: unless-stopped
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
      - JWT_SECRET_KEY=${JWT_SECRET_KEY}
      - ENVIRONMENT=production
    ports:
      - "8080:8080"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]
      interval: 30s
      timeout: 10s
      retries: 3
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

  directory-api:
    image: vpn/directory-api:${VERSION:-latest}
    restart: unless-stopped
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
      - GRPC_PORT=50051
      - ENVIRONMENT=production
    ports:
      - "8081:8081"
      - "50051:50051"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/healthz"]
      interval: 30s
      timeout: 10s
      retries: 3

  admin-api:
    image: vpn/admin-api:${VERSION:-latest}
    restart: unless-stopped
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
      - ENVIRONMENT=production
    ports:
      - "8082:8082"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8082/healthz"]
      interval: 30s
      timeout: 10s
      retries: 3

  nginx:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/ssl/certs:ro
    depends_on:
      - auth-api
      - directory-api
      - admin-api

networks:
  default:
    driver: bridge
```

#### Environment Configuration

```bash
# .env.prod
# Database Configuration
DATABASE_URL=postgresql+asyncpg://vpn_user:secure_password@postgres.example.com:5432/vpn_db

# Redis Configuration
REDIS_URL=redis://redis.example.com:6379/0

# JWT Configuration
JWT_SECRET_KEY=your-256-bit-secret-key-here
JWT_ALGORITHM=RS256
JWT_ACCESS_TOKEN_EXPIRE_MINUTES=15
JWT_REFRESH_TOKEN_EXPIRE_DAYS=7

# Service Configuration
ENVIRONMENT=production
LOG_LEVEL=INFO
CORS_ORIGINS=https://vpn.example.com,https://admin.vpn.example.com

# Monitoring
PROMETHEUS_ENABLED=true
JAEGER_ENDPOINT=http://jaeger.example.com:14268/api/traces
```

### Kubernetes Deployment

#### Namespace and ConfigMap

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: vpn-system
  labels:
    name: vpn-system

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: vpn-config
  namespace: vpn-system
data:
  environment: "production"
  log_level: "INFO"
  cors_origins: "https://vpn.example.com"
  prometheus_enabled: "true"
```

#### Secrets Management

```yaml
# k8s/secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: vpn-secrets
  namespace: vpn-system
type: Opaque
data:
  database-url: cG9zdGdyZXNxbCsuLi4=  # base64 encoded
  redis-url: cmVkaXM6Ly8uLi4=         # base64 encoded
  jwt-secret-key: eW91ci0yNTYtYml0Li4u # base64 encoded

---
apiVersion: v1
kind: Secret
metadata:
  name: tls-certificates
  namespace: vpn-system
type: kubernetes.io/tls
data:
  tls.crt: LS0tLS1CRUdJTi4uLg==      # base64 encoded certificate
  tls.key: LS0tLS1CRUdJTi4uLg==      # base64 encoded private key
```

#### Service Deployments

```yaml
# k8s/auth-api.yaml
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
              name: vpn-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: vpn-secrets
              key: redis-url
        - name: JWT_SECRET_KEY
          valueFrom:
            secretKeyRef:
              name: vpn-secrets
              key: jwt-secret-key
        - name: ENVIRONMENT
          valueFrom:
            configMapKeyRef:
              name: vpn-config
              key: environment
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

---
apiVersion: v1
kind: Service
metadata:
  name: auth-api
  namespace: vpn-system
spec:
  selector:
    app: auth-api
  ports:
  - port: 8080
    targetPort: 8080
  type: ClusterIP
```

#### Ingress Configuration

```yaml
# k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: vpn-ingress
  namespace: vpn-system
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
    nginx.ingress.kubernetes.io/rate-limit-window: "1m"
spec:
  tls:
  - hosts:
    - auth.vpn.example.com
    - directory.vpn.example.com
    - admin.vpn.example.com
    secretName: vpn-tls
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
  - host: directory.vpn.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: directory-api
            port:
              number: 8081
  - host: admin.vpn.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: admin-api
            port:
              number: 8082
```

### Cloud Provider Deployments

#### AWS ECS Deployment

```json
{
  "family": "vpn-auth-api",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "512",
  "memory": "1024",
  "executionRoleArn": "arn:aws:iam::123456789012:role/ecsTaskExecutionRole",
  "taskRoleArn": "arn:aws:iam::123456789012:role/ecsTaskRole",
  "containerDefinitions": [
    {
      "name": "auth-api",
      "image": "123456789012.dkr.ecr.us-west-2.amazonaws.com/vpn/auth-api:latest",
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "ENVIRONMENT",
          "value": "production"
        }
      ],
      "secrets": [
        {
          "name": "DATABASE_URL",
          "valueFrom": "arn:aws:secretsmanager:us-west-2:123456789012:secret:vpn/database-url"
        },
        {
          "name": "REDIS_URL", 
          "valueFrom": "arn:aws:secretsmanager:us-west-2:123456789012:secret:vpn/redis-url"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/vpn-auth-api",
          "awslogs-region": "us-west-2",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:8080/healthz || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3
      }
    }
  ]
}
```

#### Google Cloud Run Deployment

```yaml
# cloudrun/auth-api.yaml
apiVersion: serving.knative.dev/v1
kind: Service
metadata:
  name: auth-api
  annotations:
    run.googleapis.com/ingress: all
    run.googleapis.com/execution-environment: gen2
spec:
  template:
    metadata:
      annotations:
        autoscaling.knative.dev/maxScale: "10"
        run.googleapis.com/cpu-throttling: "false"
        run.googleapis.com/memory: "1Gi"
        run.googleapis.com/cpu: "1"
    spec:
      containerConcurrency: 100
      containers:
      - image: gcr.io/project-id/vpn/auth-api:latest
        ports:
        - containerPort: 8080
        env:
        - name: ENVIRONMENT
          value: "production"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: vpn-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: vpn-secrets
              key: redis-url
        resources:
          limits:
            memory: "1Gi"
            cpu: "1"
        livenessProbe:
          httpGet:
            path: /healthz
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
```

---

## Node Agent Deployment

### Standalone Server Deployment

#### Systemd Service

```bash
#!/bin/bash
# scripts/install_node_agent.sh

set -euo pipefail

NODE_VERSION=${1:-latest}
REGION_CODE=${2:-US}
REGION_CITY=${3:-"New York"}
PUBLIC_ENDPOINT=${4:-$(curl -s ifconfig.me):51820}

echo "Installing VPN Node Agent..."

# Create vpn user
sudo useradd -r -s /bin/false -d /var/lib/vpn vpn || true

# Create directories
sudo mkdir -p /opt/vpn/bin
sudo mkdir -p /var/lib/vpn
sudo mkdir -p /etc/vpn

# Download and install binary
curl -L "https://releases.vpn.example.com/node-agent/${NODE_VERSION}/node-agent-linux-amd64" \
    -o /tmp/node-agent
sudo mv /tmp/node-agent /opt/vpn/bin/
sudo chmod +x /opt/vpn/bin/node-agent
sudo chown root:root /opt/vpn/bin/node-agent

# Create configuration
sudo tee /etc/vpn/config.env << EOF
NODE_REGION_CODE=${REGION_CODE}
NODE_REGION_CITY=${REGION_CITY}
PUBLIC_ENDPOINT=${PUBLIC_ENDPOINT}
DIRECTORY_HTTP_ADDR=https://directory.vpn.example.com
DIRECTORY_GRPC_ADDR=directory.vpn.example.com:443
WG_INTERFACE=wg0
WG_LISTEN_PORT=51820
WG_ADDRESS=10.66.0.1/24
LOG_LEVEL=INFO
EOF

# Create systemd service
sudo tee /etc/systemd/system/vpn-node-agent.service << 'EOF'
[Unit]
Description=VPN Node Agent
After=network.target
Wants=network.target

[Service]
Type=simple
User=vpn
Group=vpn
ExecStart=/opt/vpn/bin/node-agent
EnvironmentFile=/etc/vpn/config.env
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=vpn-node-agent

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/vpn

# Network capabilities
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable vpn-node-agent
sudo systemctl start vpn-node-agent

# Verify installation
sleep 5
sudo systemctl status vpn-node-agent

echo "✅ VPN Node Agent installed successfully"
echo "Check status with: sudo systemctl status vpn-node-agent"
echo "View logs with: sudo journalctl -u vpn-node-agent -f"
```

### Docker Deployment

#### Production Node Container

```dockerfile
# nodes/agent/Dockerfile.prod
FROM rust:1.70-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin node-agent

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    iptables \
    iproute2 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create vpn user
RUN useradd -r -s /bin/false -d /var/lib/vpn vpn

# Copy binary
COPY --from=builder /app/target/release/node-agent /usr/local/bin/

# Create directories
RUN mkdir -p /var/lib/vpn && chown vpn:vpn /var/lib/vpn

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

USER vpn
EXPOSE 51820/udp 8080/tcp

CMD ["node-agent"]
```

#### Docker Compose for Node Fleet

```yaml
# docker-compose.nodes.yml
version: '3.8'

services:
  node-us-east-1:
    build:
      context: ./nodes/agent
      dockerfile: Dockerfile.prod
    restart: unless-stopped
    privileged: true
    network_mode: host
    environment:
      - NODE_REGION_CODE=US
      - NODE_REGION_CITY=New York
      - PUBLIC_ENDPOINT=us-east.vpn.example.com:51820
      - DIRECTORY_HTTP_ADDR=https://directory.vpn.example.com
      - DIRECTORY_GRPC_ADDR=directory.vpn.example.com:443
    volumes:
      - node_us_east_data:/var/lib/vpn
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

  node-eu-west-1:
    build:
      context: ./nodes/agent
      dockerfile: Dockerfile.prod
    restart: unless-stopped
    privileged: true
    network_mode: host
    environment:
      - NODE_REGION_CODE=DE
      - NODE_REGION_CITY=Frankfurt
      - PUBLIC_ENDPOINT=eu-west.vpn.example.com:51820
      - DIRECTORY_HTTP_ADDR=https://directory.vpn.example.com
      - DIRECTORY_GRPC_ADDR=directory.vpn.example.com:443
    volumes:
      - node_eu_west_data:/var/lib/vpn

volumes:
  node_us_east_data:
  node_eu_west_data:
```

### Kubernetes DaemonSet Deployment

```yaml
# k8s/node-agents.yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: vpn-node-agents
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
        image: vpn/node-agent:v1.0.0
        securityContext:
          privileged: true
          capabilities:
            add: ["NET_ADMIN", "NET_RAW"]
        env:
        - name: NODE_REGION_CODE
          valueFrom:
            fieldRef:
              fieldPath: metadata.labels['topology.kubernetes.io/region']
        - name: NODE_REGION_CITY
          valueFrom:
            configMapKeyRef:
              name: region-config
              key: city
        - name: PUBLIC_ENDPOINT
          valueFrom:
            fieldRef:
              fieldPath: status.hostIP
        - name: DIRECTORY_HTTP_ADDR
          value: "https://directory.vpn.example.com"
        - name: DIRECTORY_GRPC_ADDR
          value: "directory.vpn.example.com:443"
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "256Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      tolerations:
      - key: node-role.kubernetes.io/master
        operator: Exists
        effect: NoSchedule
```

---

## Database Deployment

### PostgreSQL Setup

#### Managed Database (AWS RDS)

```bash
#!/bin/bash
# scripts/setup_rds.sh

set -euo pipefail

DB_INSTANCE_ID="vpn-database"
DB_NAME="vpn_db"
DB_USERNAME="vpn_user"
DB_PASSWORD=$(openssl rand -base64 32)
VPC_ID="vpc-12345678"
SUBNET_GROUP="vpn-db-subnet-group"

echo "Creating RDS PostgreSQL instance..."

# Create DB subnet group
aws rds create-db-subnet-group \
    --db-subnet-group-name $SUBNET_GROUP \
    --db-subnet-group-description "VPN Database Subnet Group" \
    --subnet-ids subnet-12345678 subnet-87654321

# Create security group
SECURITY_GROUP_ID=$(aws ec2 create-security-group \
    --group-name vpn-database-sg \
    --description "VPN Database Security Group" \
    --vpc-id $VPC_ID \
    --query 'GroupId' --output text)

# Allow PostgreSQL access from VPC
aws ec2 authorize-security-group-ingress \
    --group-id $SECURITY_GROUP_ID \
    --protocol tcp \
    --port 5432 \
    --cidr 10.0.0.0/16

# Create RDS instance
aws rds create-db-instance \
    --db-instance-identifier $DB_INSTANCE_ID \
    --db-instance-class db.r6g.large \
    --engine postgres \
    --engine-version 15.4 \
    --master-username $DB_USERNAME \
    --master-user-password $DB_PASSWORD \
    --allocated-storage 100 \
    --storage-type gp3 \
    --storage-encrypted \
    --vpc-security-group-ids $SECURITY_GROUP_ID \
    --db-subnet-group-name $SUBNET_GROUP \
    --backup-retention-period 7 \
    --multi-az \
    --deletion-protection

echo "Database instance created: $DB_INSTANCE_ID"
echo "Username: $DB_USERNAME"
echo "Password: $DB_PASSWORD"
echo "Store password securely and update application configuration"
```

#### Self-Hosted PostgreSQL

```yaml
# docker-compose.postgres.yml
version: '3.8'

services:
  postgres:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: vpn_db
      POSTGRES_USER: vpn_user
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_INITDB_ARGS: "--auth-host=scram-sha-256"
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./postgres/init:/docker-entrypoint-initdb.d
      - ./postgres/postgresql.conf:/etc/postgresql/postgresql.conf
    ports:
      - "5432:5432"
    command: postgres -c config_file=/etc/postgresql/postgresql.conf
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U vpn_user -d vpn_db"]
      interval: 30s
      timeout: 10s
      retries: 3

  postgres-exporter:
    image: prometheuscommunity/postgres-exporter
    restart: unless-stopped
    environment:
      DATA_SOURCE_NAME: "postgresql://vpn_user:${POSTGRES_PASSWORD}@postgres:5432/vpn_db?sslmode=disable"
    ports:
      - "9187:9187"
    depends_on:
      - postgres

volumes:
  postgres_data:
    driver: local
```

#### Database Migration

```bash
#!/bin/bash
# scripts/migrate_database.sh

set -euo pipefail

DATABASE_URL=${1:-$DATABASE_URL}
MIGRATION_DIR="services/auth-api/migrations"

echo "Running database migrations..."

# Install Alembic if not present
pip install alembic psycopg2-binary

# Run migrations for each service
for service in auth-api directory-api admin-api; do
    echo "Migrating $service..."
    cd "services/$service"
    
    # Update alembic.ini with database URL
    sed -i "s|sqlalchemy.url = .*|sqlalchemy.url = $DATABASE_URL|" alembic.ini
    
    # Run migrations
    alembic upgrade head
    
    cd ../..
done

echo "✅ Database migrations completed"
```

---

## Monitoring Deployment

### Prometheus and Grafana

```yaml
# monitoring/docker-compose.yml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    restart: unless-stopped
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml
      - ./prometheus/rules:/etc/prometheus/rules
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
      - '--storage.tsdb.retention.time=30d'
      - '--web.enable-lifecycle'

  grafana:
    image: grafana/grafana:latest
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD:-admin}
      - GF_USERS_ALLOW_SIGN_UP=false
    volumes:
      - ./grafana/provisioning:/etc/grafana/provisioning
      - ./grafana/dashboards:/var/lib/grafana/dashboards
      - grafana_data:/var/lib/grafana

  alertmanager:
    image: prom/alertmanager:latest
    restart: unless-stopped
    ports:
      - "9093:9093"
    volumes:
      - ./alertmanager/alertmanager.yml:/etc/alertmanager/alertmanager.yml
      - alertmanager_data:/alertmanager

volumes:
  prometheus_data:
  grafana_data:
  alertmanager_data:
```

### Observability Configuration

```yaml
# prometheus/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "rules/*.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'vpn-services'
    static_configs:
      - targets: 
        - 'auth-api:8080'
        - 'directory-api:8081'
        - 'admin-api:8082'
    metrics_path: /metrics
    scrape_interval: 30s

  - job_name: 'node-agents'
    consul_sd_configs:
      - server: 'consul:8500'
        services: ['vpn-node-agent']
    relabel_configs:
      - source_labels: [__meta_consul_service_address]
        target_label: __address__
        replacement: '${1}:8080'

  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres-exporter:9187']

  - job_name: 'redis'
    static_configs:
      - targets: ['redis-exporter:9121']
```

---

## Security Hardening

### SSL/TLS Configuration

#### Nginx SSL Configuration

```nginx
# nginx/nginx.conf
events {
    worker_connections 1024;
}

http {
    include       /etc/nginx/mime.types;
    default_type  application/octet-stream;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    limit_req_zone $binary_remote_addr zone=auth:10m rate=5r/m;

    # Upstream servers
    upstream auth_api {
        server auth-api:8080;
        keepalive 32;
    }

    upstream directory_api {
        server directory-api:8081;
        keepalive 32;
    }

    upstream admin_api {
        server admin-api:8082;
        keepalive 32;
    }

    # HTTP to HTTPS redirect
    server {
        listen 80;
        server_name auth.vpn.example.com directory.vpn.example.com admin.vpn.example.com;
        return 301 https://$server_name$request_uri;
    }

    # Auth API
    server {
        listen 443 ssl http2;
        server_name auth.vpn.example.com;

        ssl_certificate /etc/ssl/certs/auth.vpn.example.com.crt;
        ssl_certificate_key /etc/ssl/certs/auth.vpn.example.com.key;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512;
        ssl_prefer_server_ciphers off;

        location / {
            limit_req zone=api burst=20 nodelay;
            
            proxy_pass http://auth_api;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        location /auth/login {
            limit_req zone=auth burst=5 nodelay;
            
            proxy_pass http://auth_api;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
    }

    # Directory API
    server {
        listen 443 ssl http2;
        server_name directory.vpn.example.com;

        ssl_certificate /etc/ssl/certs/directory.vpn.example.com.crt;
        ssl_certificate_key /etc/ssl/certs/directory.vpn.example.com.key;
        ssl_protocols TLSv1.2 TLSv1.3;

        location / {
            limit_req zone=api burst=20 nodelay;
            
            proxy_pass http://directory_api;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
    }
}

# gRPC configuration for directory API
stream {
    upstream directory_grpc {
        server directory-api:50051;
    }

    server {
        listen 443 ssl;
        ssl_certificate /etc/ssl/certs/directory.vpn.example.com.crt;
        ssl_certificate_key /etc/ssl/certs/directory.vpn.example.com.key;
        ssl_protocols TLSv1.2 TLSv1.3;
        
        proxy_pass directory_grpc;
        proxy_timeout 1s;
        proxy_responses 1;
        error_log /var/log/nginx/grpc.log;
    }
}
```

### Firewall Configuration

#### iptables Rules

```bash
#!/bin/bash
# scripts/setup_firewall.sh

set -euo pipefail

echo "Configuring firewall rules..."

# Flush existing rules
iptables -F
iptables -X
iptables -t nat -F
iptables -t nat -X

# Default policies
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT ACCEPT

# Allow loopback
iptables -A INPUT -i lo -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT

# Allow established connections
iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT

# Allow SSH (change port as needed)
iptables -A INPUT -p tcp --dport 22 -m conntrack --ctstate NEW -j ACCEPT

# Allow HTTP/HTTPS
iptables -A INPUT -p tcp --dport 80 -j ACCEPT
iptables -A INPUT -p tcp --dport 443 -j ACCEPT

# Allow WireGuard
iptables -A INPUT -p udp --dport 51820 -j ACCEPT

# Allow internal service communication
iptables -A INPUT -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -s 172.16.0.0/12 -j ACCEPT
iptables -A INPUT -s 192.168.0.0/16 -j ACCEPT

# Rate limiting for SSH
iptables -A INPUT -p tcp --dport 22 -m recent --name ssh --set
iptables -A INPUT -p tcp --dport 22 -m recent --name ssh --rcheck --seconds 60 --hitcount 4 -j DROP

# Log dropped packets
iptables -A INPUT -j LOG --log-prefix "DROPPED: "

# Save rules
iptables-save > /etc/iptables/rules.v4

echo "✅ Firewall configured"
```

---

## Automated Deployment

### Terraform Infrastructure

#### AWS Infrastructure

```hcl
# terraform/aws/main.tf
terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

# VPC and Networking
module "vpc" {
  source = "terraform-aws-modules/vpc/aws"
  
  name = "vpn-vpc"
  cidr = "10.0.0.0/16"
  
  azs             = ["${var.aws_region}a", "${var.aws_region}b", "${var.aws_region}c"]
  private_subnets = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
  public_subnets  = ["10.0.101.0/24", "10.0.102.0/24", "10.0.103.0/24"]
  
  enable_nat_gateway = true
  enable_vpn_gateway = false
  
  tags = {
    Project = "vpn-mvp"
  }
}

# ECS Cluster
resource "aws_ecs_cluster" "vpn_cluster" {
  name = "vpn-cluster"
  
  setting {
    name  = "containerInsights"
    value = "enabled"
  }
}

# Application Load Balancer
resource "aws_lb" "vpn_alb" {
  name               = "vpn-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = module.vpc.public_subnets
  
  enable_deletion_protection = true
  
  tags = {
    Project = "vpn-mvp"
  }
}

# RDS Database
resource "aws_db_instance" "vpn_db" {
  identifier = "vpn-database"
  
  engine         = "postgres"
  engine_version = "15.4"
  instance_class = "db.r6g.large"
  
  allocated_storage     = 100
  max_allocated_storage = 1000
  storage_type          = "gp3"
  storage_encrypted     = true
  
  db_name  = "vpn_db"
  username = "vpn_user"
  password = var.db_password
  
  vpc_security_group_ids = [aws_security_group.rds.id]
  db_subnet_group_name   = aws_db_subnet_group.vpn_db.name
  
  backup_retention_period = 7
  backup_window          = "03:00-04:00"
  maintenance_window     = "sun:04:00-sun:05:00"
  
  multi_az               = true
  deletion_protection    = true
  
  tags = {
    Project = "vpn-mvp"
  }
}

# ElastiCache Redis
resource "aws_elasticache_subnet_group" "vpn_redis" {
  name       = "vpn-redis-subnet-group"
  subnet_ids = module.vpc.private_subnets
}

resource "aws_elasticache_replication_group" "vpn_redis" {
  replication_group_id       = "vpn-redis"
  description                = "VPN Redis cluster"
  
  node_type                  = "cache.r6g.large"
  port                       = 6379
  parameter_group_name       = "default.redis7"
  
  num_cache_clusters         = 2
  automatic_failover_enabled = true
  multi_az_enabled          = true
  
  subnet_group_name = aws_elasticache_subnet_group.vpn_redis.name
  security_group_ids = [aws_security_group.redis.id]
  
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  
  tags = {
    Project = "vpn-mvp"
  }
}
```

#### Ansible Playbooks

```yaml
# ansible/deploy.yml
---
- name: Deploy VPN Infrastructure
  hosts: all
  become: yes
  vars:
    vpn_version: "{{ version | default('latest') }}"
    environment: "{{ env | default('production') }}"
  
  tasks:
    - name: Update system packages
      apt:
        update_cache: yes
        upgrade: dist
        
    - name: Install Docker
      apt:
        name: 
          - docker.io
          - docker-compose
        state: present
        
    - name: Start Docker service
      systemd:
        name: docker
        state: started
        enabled: yes
        
    - name: Create VPN directories
      file:
        path: "{{ item }}"
        state: directory
        owner: root
        group: root
        mode: '0755'
      loop:
        - /opt/vpn
        - /opt/vpn/config
        - /opt/vpn/ssl
        - /var/log/vpn
        
    - name: Copy SSL certificates
      copy:
        src: "{{ item.src }}"
        dest: "{{ item.dest }}"
        owner: root
        group: root
        mode: '0600'
      loop:
        - { src: "ssl/{{ inventory_hostname }}.crt", dest: "/opt/vpn/ssl/server.crt" }
        - { src: "ssl/{{ inventory_hostname }}.key", dest: "/opt/vpn/ssl/server.key" }
        
    - name: Template configuration files
      template:
        src: "{{ item.src }}"
        dest: "{{ item.dest }}"
        owner: root
        group: root
        mode: '0644'
      loop:
        - { src: "docker-compose.yml.j2", dest: "/opt/vpn/docker-compose.yml" }
        - { src: "env.j2", dest: "/opt/vpn/.env" }
      notify: restart vpn services
      
    - name: Pull Docker images
      docker_image:
        name: "{{ item }}"
        source: pull
      loop:
        - "vpn/auth-api:{{ vpn_version }}"
        - "vpn/directory-api:{{ vpn_version }}"
        - "vpn/admin-api:{{ vpn_version }}"
        
    - name: Start VPN services
      docker_compose:
        project_src: /opt/vpn
        state: present
        
  handlers:
    - name: restart vpn services
      docker_compose:
        project_src: /opt/vpn
        restarted: yes
```

### CI/CD Pipeline

#### GitHub Actions Deployment

```yaml
# .github/workflows/deploy.yml
name: Deploy to Production

on:
  push:
    tags: ['v*']

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout code
        uses: actions/checkout@v3
        
      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v2
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-west-2
          
      - name: Login to Amazon ECR
        id: login-ecr
        uses: aws-actions/amazon-ecr-login@v1
        
      - name: Build and push images
        env:
          ECR_REGISTRY: ${{ steps.login-ecr.outputs.registry }}
          IMAGE_TAG: ${{ github.ref_name }}
        run: |
          # Build and push auth-api
          docker build -t $ECR_REGISTRY/vpn/auth-api:$IMAGE_TAG services/auth-api
          docker push $ECR_REGISTRY/vpn/auth-api:$IMAGE_TAG
          
          # Build and push directory-api
          docker build -t $ECR_REGISTRY/vpn/directory-api:$IMAGE_TAG services/directory-api
          docker push $ECR_REGISTRY/vpn/directory-api:$IMAGE_TAG
          
          # Build and push admin-api
          docker build -t $ECR_REGISTRY/vpn/admin-api:$IMAGE_TAG services/admin-api
          docker push $ECR_REGISTRY/vpn/admin-api:$IMAGE_TAG
          
          # Build and push node-agent
          docker build -t $ECR_REGISTRY/vpn/node-agent:$IMAGE_TAG nodes/agent
          docker push $ECR_REGISTRY/vpn/node-agent:$IMAGE_TAG

  deploy-staging:
    needs: build-and-push
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - name: Deploy to staging
        run: |
          # Update ECS services with new image tags
          aws ecs update-service \
            --cluster vpn-staging-cluster \
            --service auth-api \
            --force-new-deployment
            
  deploy-production:
    needs: [build-and-push, deploy-staging]
    runs-on: ubuntu-latest
    environment: production
    steps:
      - name: Deploy to production
        run: |
          # Blue-green deployment using ECS
          aws ecs update-service \
            --cluster vpn-production-cluster \
            --service auth-api \
            --task-definition auth-api:${{ github.ref_name }} \
            --force-new-deployment
```

---

## Troubleshooting Deployment Issues

### Common Issues and Solutions

#### Service Health Checks Failing

```bash
# Check service logs
docker logs vpn-auth-api-1 --tail=100

# Check database connectivity
docker exec vpn-auth-api-1 curl -f $DATABASE_URL

# Check Redis connectivity
docker exec vpn-auth-api-1 redis-cli -u $REDIS_URL ping

# Verify environment variables
docker exec vpn-auth-api-1 env | grep -E "(DATABASE|REDIS|JWT)"
```

#### Database Migration Issues

```bash
# Check migration status
cd services/auth-api
alembic current

# Show migration history
alembic history --verbose

# Force migration to specific revision
alembic stamp head

# Manual migration rollback
alembic downgrade -1
```

#### Node Agent Registration Problems

```bash
# Check node agent logs
sudo journalctl -u vpn-node-agent -f

# Test directory API connectivity
curl -f https://directory.vpn.example.com/healthz

# Check WireGuard interface
sudo wg show

# Verify iptables rules
sudo iptables -L -n -v
```

### Deployment Validation

#### Health Check Script

```bash
#!/bin/bash
# scripts/validate_deployment.sh

set -euo pipefail

BASE_URL=${1:-https://vpn.example.com}
TIMEOUT=30

echo "🔍 Validating VPN deployment..."

# Check service health endpoints
services=("auth" "directory" "admin")
for service in "${services[@]}"; do
    echo "Checking $service API..."
    if curl -f --max-time $TIMEOUT "$BASE_URL:8080/healthz" > /dev/null 2>&1; then
        echo "✅ $service API is healthy"
    else
        echo "❌ $service API is not responding"
        exit 1
    fi
done

# Check database connectivity
echo "Checking database..."
if docker exec vpn-auth-api-1 python -c "
import asyncpg
import asyncio
async def test():
    conn = await asyncpg.connect('$DATABASE_URL')
    await conn.execute('SELECT 1')
    await conn.close()
asyncio.run(test())
" > /dev/null 2>&1; then
    echo "✅ Database is accessible"
else
    echo "❌ Database connection failed"
    exit 1
fi

# Check Redis connectivity
echo "Checking Redis..."
if docker exec vpn-auth-api-1 redis-cli -u "$REDIS_URL" ping | grep -q PONG; then
    echo "✅ Redis is accessible"
else
    echo "❌ Redis connection failed"
    exit 1
fi

# Test user registration flow
echo "Testing user registration..."
response=$(curl -s -X POST "$BASE_URL/users/register" \
    -H "Content-Type: application/json" \
    -d '{
        "email": "test@example.com",
        "password": "TestPassword123!",
        "consent": true,
        "tos_version": "v1.0"
    }')

if echo "$response" | grep -q "user_id"; then
    echo "✅ User registration works"
else
    echo "❌ User registration failed: $response"
    exit 1
fi

echo "🎉 Deployment validation completed successfully!"
```

---

For additional deployment scenarios and advanced configurations, see:
- [Infrastructure Documentation](../infra/INDEX.md)
- [Operations Guide](../ops/INDEX.md)
- [Security Documentation](../security/)
- [Architecture Overview](../architecture/ARCHITECTURE.md)
