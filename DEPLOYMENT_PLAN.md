# VPN MVP - Linux Root Server Deployment Plan

## Overview

This guide walks through deploying the VPN MVP services and node agents to a Linux root server. The deployment includes:
- All backend services (auth-api, directory-api, admin-api, postgres, redis)
- Two VPN node agents (Berlin and Munich)
- Monitoring stack (Prometheus, Grafana)

## Prerequisites

### Server Requirements

- **OS**: Linux (Ubuntu 22.04+ or Debian 11+ recommended)
- **Kernel**: Linux 5.6+ with WireGuard support
- **RAM**: Minimum 4GB (8GB+ recommended)
- **Storage**: 20GB+ free space
- **Network**: 
  - Public IP address
  - UDP ports: 51820, 51821 (for WireGuard nodes)
  - TCP ports: 8080 (auth-api), 8081 (directory-api), 8082 (admin-api)
  - Ports 5432, 6379 should be firewalled (internal only)

### Software Prerequisites

```bash
# Install Docker and Docker Compose
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER

# Install Docker Compose Plugin (if not included)
sudo apt-get update
sudo apt-get install docker-compose-plugin

# Verify installation
docker --version
docker compose version

# Install WireGuard tools
sudo apt-get install wireguard-tools

# Verify WireGuard kernel module
sudo modprobe wireguard
lsmod | grep wireguard
```

## Step 1: Clone Repository on Linux Server

```bash
# SSH into your root server
ssh user@your-server-ip

# Clone the repository
git clone https://github.com/yourusername/vpn_mvp.git
cd vpn_mvp
git checkout refactor/wireguard-rs
```

## Step 2: Configure Environment Variables

### Create Production Environment File

```bash
# Copy example env files
cp services/auth-api/env.example services/auth-api/.env
cp services/directory-api/env.example services/directory-api/.env
cp services/admin-api/env.example services/admin-api/.env

# Edit each .env file with production values
# Key values to change:
# - Database passwords (strong random passwords)
# - JWT secrets (strong random strings)
# - Server URLs/IPs
```

### Key Environment Variables to Set

**For `services/auth-api/.env`:**
```env
DATABASE_URL=postgresql://postgres:STRONG_PASSWORD@postgres:5432/postgres
REDIS_URL=redis://redis:6379/0
JWT_SECRET=your-very-strong-jwt-secret-here
JWT_EXPIRES_IN=3600
```

**For `services/directory-api/.env`:**
```env
DATABASE_URL=postgresql://postgres:STRONG_PASSWORD@postgres:5432/postgres
REDIS_URL=redis://redis:6379/0
AUTH_API_URL=http://auth-api:8080
```

**For `services/admin-api/.env`:**
```env
DATABASE_URL=postgresql://postgres:STRONG_PASSWORD@postgres:5432/postgres
AUTH_API_URL=http://auth-api:8080
DIRECTORY_API_URL=http://directory-api:8081
ADMIN_API_URL=http://admin-api:8082
```

## Step 3: Create Production Docker Compose File

Create `docker-compose.prod.yml` for production deployment:

```bash
cat > docker-compose.prod.yml << 'EOF'
# Production Docker Compose Configuration

x-node-agent-env: &node-agent-env
  DIRECTORY_HTTP_ADDR: http://directory-api:8081
  DIRECTORY_GRPC_ADDR: http://directory-api:50051
  RUN_NFT_SETUP: "1"
  WG_SERVER_KEY_PATH: /var/run/vpn/server.key
  MTLS_ENABLED: "0"  # Set to 1 if using mTLS in production

x-node-agent-base: &node-agent-base
  build: ./nodes/agent
  network_mode: host  # CRITICAL: Use host networking on Linux
  cap_add:
    - NET_ADMIN
    - SYS_MODULE
  tmpfs:
    - /etc/wireguard:rw,noexec,nosuid,nodev,mode=0700
    - /var/run/vpn:rw,noexec,nosuid,nodev,mode=0700
  depends_on:
    directory-api:
      condition: service_healthy
    directory-seed:
      condition: service_completed_successfully
  volumes:
    - ./tools/pki/dev:/certs:ro  # Update to production certs path
  environment:
    <<: *node-agent-env
  restart: unless-stopped

services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:-change-me-in-production}
      POSTGRES_USER: ${POSTGRES_USER:-postgres}
      POSTGRES_DB: ${POSTGRES_DB:-postgres}
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER:-postgres}"]
      interval: 5s
      retries: 10
    restart: unless-stopped
    networks:
      - vpn-internal

  redis:
    image: redis:7
    command: ["redis-server", "--appendonly", "no"]
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      retries: 10
    restart: unless-stopped
    networks:
      - vpn-internal

  auth-api:
    build:
      context: .
      dockerfile: ./services/auth-api/Dockerfile
    env_file: ./services/auth-api/.env
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    ports:
      - "8080:8080"
    healthcheck:
      test: ["CMD", "python", "-c", "import json,urllib.request,sys; sys.exit(0 if json.load(urllib.request.urlopen('http://localhost:8080/healthz')).get('status')=='ok' else 1)"]
      interval: 5s
      retries: 10
    restart: unless-stopped
    networks:
      - vpn-internal

  directory-api:
    build:
      context: .
      dockerfile: ./services/directory-api/Dockerfile
    env_file: ./services/directory-api/.env
    depends_on:
      - postgres
      - redis
      - auth-api
    ports:
      - "8081:8081"
      - "50051:50051"
    environment:
      - GRPC_TLS_ENABLED=${GRPC_TLS_ENABLED:-0}
      - GRPC_TLS_CERT_PATH=/certs/directory.crt
      - GRPC_TLS_KEY_PATH=/certs/directory.key
      - GRPC_TLS_CLIENT_CA_PATH=/certs/ca.crt
    volumes:
      - ./tools/pki/dev:/certs:ro
    healthcheck:
      test: ["CMD", "python", "-c", "import json,urllib.request,sys; sys.exit(0 if json.load(urllib.request.urlopen('http://localhost:8081/healthz')).get('status')=='ok' else 1)"]
      interval: 5s
      retries: 10
    restart: unless-stopped
    networks:
      - vpn-internal

  admin-api:
    build:
      context: .
      dockerfile: ./services/admin-api/Dockerfile
    env_file: ./services/admin-api/.env
    depends_on:
      - auth-api
      - directory-api
    ports:
      - "8082:8082"
    healthcheck:
      test: ["CMD", "python", "-c", "import json,urllib.request,sys; sys.exit(0 if json.load(urllib.request.urlopen('http://localhost:8082/healthz')).get('status')=='ok' else 1)"]
      interval: 5s
      retries: 10
    restart: unless-stopped
    networks:
      - vpn-internal

  directory-seed:
    image: python:3.12-slim
    depends_on:
      directory-api:
        condition: service_healthy
    volumes:
      - ./services/directory-api/scripts:/scripts:ro
    environment:
      DIRECTORY_URL: http://directory-api:8081
    entrypoint:
      - "sh"
      - "-c"
      - "pip install --no-cache-dir requests && python /scripts/seed_regions.py"
    restart: "no"
    networks:
      - vpn-internal

  node-agent-de-berlin:
    <<: *node-agent-base
    container_name: node-agent-de-berlin
    environment:
      <<: *node-agent-env
      PUBLIC_ENDPOINT: YOUR_SERVER_IP:51820  # Replace with your server's public IP
      NODE_REGION_CODE: DE
      NODE_REGION_CITY: Berlin
      NODE_EGRESS_IPS: YOUR_SERVER_IP  # Replace with your server's public IP
      WG_ADDRESS: 10.66.0.1/24
      WG_LISTEN_PORT: 51820

  node-agent-de-munich:
    <<: *node-agent-base
    container_name: node-agent-de-munich
    environment:
      <<: *node-agent-env
      PUBLIC_ENDPOINT: YOUR_SERVER_IP:51821  # Replace with your server's public IP
      NODE_REGION_CODE: DE
      NODE_REGION_CITY: Munich
      NODE_EGRESS_IPS: YOUR_SERVER_IP  # Replace with your server's public IP
      WG_ADDRESS: 10.66.0.1/24
      WG_LISTEN_PORT: 51821

networks:
  vpn-internal:
    driver: bridge

volumes:
  postgres-data:
EOF
```

**Important**: Replace `YOUR_SERVER_IP` with your server's actual public IP address.

## Step 4: Configure Firewall (ufw)

```bash
# Allow SSH
sudo ufw allow 22/tcp

# Allow VPN control plane APIs
sudo ufw allow 8080/tcp  # auth-api
sudo ufw allow 8081/tcp  # directory-api
sudo ufw allow 8082/tcp  # admin-api

# Allow WireGuard UDP ports
sudo ufw allow 51820/udp
sudo ufw allow 51821/udp

# Enable firewall
sudo ufw enable

# Verify rules
sudo ufw status verbose
```

## Step 5: Build and Deploy

```bash
# Build all images
docker compose -f docker-compose.prod.yml build

# Start services (excluding node agents first to verify control plane)
docker compose -f docker-compose.prod.yml up -d postgres redis auth-api directory-api admin-api directory-seed

# Wait for services to be healthy
docker compose -f docker-compose.prod.yml ps

# Check logs to ensure services started correctly
docker compose -f docker-compose.prod.yml logs -f auth-api directory-api

# Once control plane is healthy, start node agents
docker compose -f docker-compose.prod.yml up -d node-agent-de-berlin node-agent-de-munich

# Verify all services are running
docker compose -f docker-compose.prod.yml ps
```

## Step 6: Verify Deployment

### Check Node Agent Status

```bash
# Check WireGuard interfaces
docker exec node-agent-de-berlin wg show wg0
docker exec node-agent-de-munich wg show wg0

# Check nftables rules
docker exec node-agent-de-berlin nft list table inet vpn

# Check node agent logs
docker logs -f node-agent-de-berlin
docker logs -f node-agent-de-munich
```

### Test Control Plane APIs

```bash
# Test auth-api
curl http://localhost:8080/healthz

# Test directory-api
curl http://localhost:8081/healthz

# Test admin-api
curl http://localhost:8082/healthz

# List regions (should show Berlin and Munich)
curl http://localhost:8081/regions
```

### Verify Forwarding is Working

```bash
# Check IPv4 forwarding is enabled
sysctl net.ipv4.ip_forward  # Should be 1

# Check nftables forward chain counters
docker exec node-agent-de-berlin nft list table inet vpn -a | grep counter

# Try connecting a client and check if counters increment
```

## Step 7: Update Client Configuration

Update your client to point to the production server:

1. **Set server host** in the client UI to your server's IP or domain
2. **Test connection** to verify end-to-end functionality
3. **Test VPN connectivity**:
   - Connect to VPN
   - Verify you can ping `8.8.8.8`
   - Verify you can access websites
   - Check your public IP (should match server's IP)

## Troubleshooting

### Node Agents Not Starting

```bash
# Check container logs
docker logs node-agent-de-berlin
docker logs node-agent-de-munich

# Verify WireGuard kernel module
modprobe wireguard
lsmod | grep wireguard

# Check if ports are already in use
sudo netstat -tulpn | grep 51820
sudo netstat -tulpn | grep 51821
```

### VPN Forwarding Not Working

```bash
# Verify host networking mode
docker inspect node-agent-de-berlin | grep NetworkMode  # Should be "host"

# Check IPv4 forwarding
sysctl net.ipv4.ip_forward  # Should be 1

# Check nftables rules
docker exec node-agent-de-berlin nft list table inet vpn

# Verify routing on host
ip route show
```

### Control Plane Connection Issues

```bash
# Test connectivity from node agent to directory-api
docker exec node-agent-de-berlin curl http://directory-api:8081/healthz

# Check DNS resolution
docker exec node-agent-de-berlin nslookup directory-api

# Verify network connectivity
docker exec node-agent-de-berlin ping -c 3 directory-api
```

## Security Considerations

1. **Change default passwords** in `.env` files
2. **Use strong JWT secrets** (generate with: `openssl rand -hex 32`)
3. **Enable mTLS** for production (`MTLS_ENABLED=1`)
4. **Use production certificates** (replace `tools/pki/dev` mounts)
5. **Restrict firewall** to only necessary ports
6. **Regular updates**: Keep Docker images and system updated
7. **Monitor logs** for suspicious activity
8. **Backup database** regularly

## Monitoring

### View Logs

```bash
# All services
docker compose -f docker-compose.prod.yml logs -f

# Specific service
docker compose -f docker-compose.prod.yml logs -f node-agent-de-berlin

# Last 100 lines
docker compose -f docker-compose.prod.yml logs --tail=100 node-agent-de-berlin
```

### Resource Usage

```bash
# Container stats
docker stats

# Disk usage
docker system df
```

## Maintenance

### Update Services

```bash
# Pull latest code
git pull origin refactor/wireguard-rs

# Rebuild and restart
docker compose -f docker-compose.prod.yml up -d --build

# Restart specific service
docker compose -f docker-compose.prod.yml restart node-agent-de-berlin
```

### Backup Database

```bash
# Create backup
docker exec postgres pg_dumpall -U postgres > backup_$(date +%Y%m%d_%H%M%S).sql

# Restore backup
cat backup_file.sql | docker exec -i postgres psql -U postgres
```

## Remote Debugging Setup

See `REMOTE_DEBUGGING.md` for instructions on setting up remote debugging capabilities.

