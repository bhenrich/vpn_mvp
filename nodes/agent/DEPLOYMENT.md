# node-agent – Production Deployment Guide

**Service role**: Data-plane agent running on VPN nodes, managing WireGuard interfaces and connecting to `directory-api` over HTTP/gRPC.

## Overview

The `node-agent` is a containerized Rust service that:
- Self-registers with the `directory-api` control plane
- Manages WireGuard interfaces and peer configurations
- Streams dynamic peer updates via gRPC
- Sends periodic heartbeats to report node status
- Handles DNS resolution via Unbound
- Configures firewall rules via nftables

## Building the Production Image

### From Repository Root

```bash
docker build -t node-agent:latest ./nodes/agent
```

### Tagging for Registry

```bash
docker tag node-agent:latest registry.example.com/vpn/node-agent:v1.0.0
docker push registry.example.com/vpn/node-agent:v1.0.0
```

## Host Requirements

### Linux Kernel

- **WireGuard support**: Kernel module or built-in support (Linux 5.6+)
- Verify with: `modprobe wireguard && lsmod | grep wireguard`

### System Capabilities

The container requires:
- `NET_ADMIN`: To create/manage network interfaces
- `SYS_MODULE`: To load WireGuard kernel module (if needed)

### Network Configuration

- **IPv4 forwarding**: Enabled automatically by the container entrypoint
- **Firewall**: nftables or iptables support (container configures rules automatically)
- **Port access**: UDP port for WireGuard (default 51820) must be accessible from the internet

## Required Environment Variables

### Control Plane Connection

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DIRECTORY_HTTP_ADDR` | No | `http://127.0.0.1:8081` | Base URL for directory-api HTTP endpoints |
| `DIRECTORY_GRPC_ADDR` | No | `http://127.0.0.1:50051` | gRPC address for config streaming |

### Node Identity

| Variable | Required | Description |
|----------|----------|-------------|
| `PUBLIC_ENDPOINT` | **Yes** | Hostname or IP:port that clients will connect to (e.g., `vpn-us-west.example.com` or `203.0.113.10:51820`) |
| `NODE_REGION_ID` | Conditional* | UUID of the region this node belongs to |
| `NODE_REGION_CODE` | Conditional* | ISO-3166-1 alpha-2 country code (e.g., `US`, `DE`) |
| `NODE_REGION_CITY` | Conditional* | City name (e.g., `San Francisco`, `Berlin`) |
| `NODE_EGRESS_IPS` | No | Comma-separated list of egress IPs advertised to clients (e.g., `203.0.113.10,203.0.113.11`) |

\* Either `NODE_REGION_ID` must be set, or both `NODE_REGION_CODE` and `NODE_REGION_CITY` must be provided.

### WireGuard Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `WG_INTERFACE` | No | `wg0` | WireGuard interface name |
| `WG_ADDRESS` | No | `10.66.0.1/24` | WireGuard interface IP address and prefix |
| `WG_LISTEN_PORT` | No | `51820` | UDP port to listen on for WireGuard traffic |
| `WG_SERVER_KEY_PATH` | No | `/var/run/vpn/server.key` | Path to store/load the WireGuard server private key |

### Security (mTLS)

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `MTLS_ENABLED` | No | `0` | Set to `1` to enable mutual TLS for gRPC connections |
| `MTLS_CA_CERT_PATH` | Conditional** | - | Path to CA certificate for verifying directory-api |
| `MTLS_CLIENT_CERT_PATH` | Conditional** | - | Path to client certificate for node authentication |
| `MTLS_CLIENT_KEY_PATH` | Conditional** | - | Path to client private key |

\** Required when `MTLS_ENABLED=1`

### Optional Features

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUN_NFT_SETUP` | No | `0` | Set to `1` to apply nftables firewall rules on startup |
| `ENABLE_ADMISSION` | No | `0` | Set to `1` to enable admission control server (future feature) |

## Production Deployment Examples

### Docker Run (Standalone)

```bash
docker run -d \
  --name node-agent \
  --cap-add NET_ADMIN \
  --cap-add SYS_MODULE \
  --tmpfs /etc/wireguard:rw,noexec,nosuid,nodev,mode=0700 \
  --tmpfs /var/run/vpn:rw,noexec,nosuid,nodev,mode=0700 \
  -p 51820:51820/udp \
  -e DIRECTORY_HTTP_ADDR=https://directory-api.example.com:8081 \
  -e DIRECTORY_GRPC_ADDR=https://directory-api.example.com:50051 \
  -e PUBLIC_ENDPOINT=vpn-us-west.example.com:51820 \
  -e NODE_REGION_CODE=US \
  -e NODE_REGION_CITY="San Francisco" \
  -e NODE_EGRESS_IPS=203.0.113.10 \
  -e WG_LISTEN_PORT=51820 \
  -e RUN_NFT_SETUP=1 \
  -e MTLS_ENABLED=1 \
  -v /path/to/certs:/certs:ro \
  -e MTLS_CA_CERT_PATH=/certs/ca.crt \
  -e MTLS_CLIENT_CERT_PATH=/certs/node.crt \
  -e MTLS_CLIENT_KEY_PATH=/certs/node.key \
  node-agent:latest
```

### Docker Compose (Production)

Create `docker-compose.prod.yml`:

```yaml
version: "3.9"

services:
  node-agent:
    image: registry.example.com/vpn/node-agent:v1.0.0
    container_name: node-agent-us-west
    restart: unless-stopped
    cap_add:
      - NET_ADMIN
      - SYS_MODULE
    tmpfs:
      - /etc/wireguard:rw,noexec,nosuid,nodev,mode=0700
      - /var/run/vpn:rw,noexec,nosuid,nodev,mode=0700
    ports:
      - "51820:51820/udp"
    environment:
      DIRECTORY_HTTP_ADDR: https://directory-api.example.com:8081
      DIRECTORY_GRPC_ADDR: https://directory-api.example.com:50051
      PUBLIC_ENDPOINT: vpn-us-west.example.com:51820
      NODE_REGION_CODE: US
      NODE_REGION_CITY: "San Francisco"
      NODE_EGRESS_IPS: 203.0.113.10
      WG_LISTEN_PORT: 51820
      RUN_NFT_SETUP: "1"
      MTLS_ENABLED: "1"
      MTLS_CA_CERT_PATH: /certs/ca.crt
      MTLS_CLIENT_CERT_PATH: /certs/node.crt
      MTLS_CLIENT_KEY_PATH: /certs/node.key
    volumes:
      - ./certs:/certs:ro
    healthcheck:
      test: ["CMD", "wg", "show", "wg0"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
```

Deploy with:

```bash
docker compose -f docker-compose.prod.yml up -d
```

### Systemd Service

Create `/etc/systemd/system/node-agent.service`:

```ini
[Unit]
Description=VPN Node Agent
After=docker.service
Requires=docker.service

[Service]
Type=notify
ExecStart=/usr/bin/docker run --rm \
  --name node-agent \
  --cap-add NET_ADMIN \
  --cap-add SYS_MODULE \
  --tmpfs /etc/wireguard:rw,noexec,nosuid,nodev,mode=0700 \
  --tmpfs /var/run/vpn:rw,noexec,nosuid,nodev,mode=0700 \
  -p 51820:51820/udp \
  -e DIRECTORY_HTTP_ADDR=https://directory-api.example.com:8081 \
  -e DIRECTORY_GRPC_ADDR=https://directory-api.example.com:50051 \
  -e PUBLIC_ENDPOINT=vpn-us-west.example.com:51820 \
  -e NODE_REGION_CODE=US \
  -e NODE_REGION_CITY="San Francisco" \
  -e NODE_EGRESS_IPS=203.0.113.10 \
  -e WG_LISTEN_PORT=51820 \
  -e RUN_NFT_SETUP=1 \
  -e MTLS_ENABLED=1 \
  -v /etc/vpn/certs:/certs:ro \
  -e MTLS_CA_CERT_PATH=/certs/ca.crt \
  -e MTLS_CLIENT_CERT_PATH=/certs/node.crt \
  -e MTLS_CLIENT_KEY_PATH=/certs/node.key \
  registry.example.com/vpn/node-agent:v1.0.0
ExecStop=/usr/bin/docker stop node-agent
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable node-agent
sudo systemctl start node-agent
sudo systemctl status node-agent
```

## Security Best Practices

### Key Management

- **Use tmpfs mounts**: WireGuard keys are stored in memory-only filesystems (`/var/run/vpn`)
- **Key rotation**: Keys are auto-generated on first run. For production, consider:
  - Pre-generating keys securely
  - Mounting keys from a secret manager (HashiCorp Vault, AWS Secrets Manager, etc.)
  - Using read-only mounts with restricted permissions

### Network Isolation

- **Firewall rules**: The container automatically configures nftables rules when `RUN_NFT_SETUP=1`
- **Host firewall**: Ensure the host firewall allows UDP traffic on the WireGuard port
- **Network namespace**: Consider using Docker's network isolation features

### mTLS Configuration

- **Enable mTLS in production**: Set `MTLS_ENABLED=1`
- **Certificate rotation**: Plan for regular certificate rotation
- **Secure storage**: Store certificates in read-only volumes with minimal permissions

### Container Security

- **Read-only root filesystem**: Consider `--read-only` with appropriate tmpfs mounts
- **User namespace**: Run as non-root user if possible (may require additional capabilities)
- **Resource limits**: Set memory and CPU limits appropriate for your workload

## Health Checks and Monitoring

### Container Health Check

The agent sends heartbeats to `directory-api` every 30 seconds. Monitor via:

```bash
# Check if WireGuard interface is up
docker exec node-agent wg show wg0

# Check container logs
docker logs -f node-agent

# Check container health (if healthcheck configured)
docker inspect --format='{{.State.Health.Status}}' node-agent
```

### External Monitoring

- **directory-api**: Tracks node status via heartbeats. Query the admin API to check node health
- **Prometheus**: Future versions may export metrics
- **Log aggregation**: Forward container logs to your logging infrastructure

### Troubleshooting

**Node not registering:**
- Verify `DIRECTORY_HTTP_ADDR` is reachable from the container
- Check that `PUBLIC_ENDPOINT`, `NODE_REGION_CODE`, and `NODE_REGION_CITY` are set correctly
- Review container logs: `docker logs node-agent`

**WireGuard interface not coming up:**
- Verify kernel has WireGuard support: `modprobe wireguard`
- Check container capabilities: `NET_ADMIN` and `SYS_MODULE` are required
- Review logs for interface creation errors

**Clients cannot connect:**
- Verify UDP port is open and accessible: `nc -u -v <PUBLIC_ENDPOINT> <PORT>`
- Check firewall rules: `docker exec node-agent nft list ruleset`
- Verify DNS resolution: `docker exec node-agent unbound-control status`

**gRPC stream disconnects:**
- Check mTLS certificates are valid and mounted correctly
- Verify `DIRECTORY_GRPC_ADDR` is reachable
- Review logs for connection errors

## Scaling and Updates

### Single Instance Per Node

Each physical server should run **one** `node-agent` container. Multiple agents on the same host will conflict on the WireGuard interface.

### Rolling Updates

1. **Graceful shutdown**: The agent handles SIGTERM and cleans up gracefully
2. **Drain connections**: Future versions may support drain signals via `directory-api`
3. **Zero-downtime**: Use orchestration (Kubernetes, Docker Swarm) with rolling update strategies

### Multi-Region Deployment

Deploy separate containers for each region:

```bash
# US West
docker run ... -e NODE_REGION_CODE=US -e NODE_REGION_CITY="San Francisco" ...

# EU Central
docker run ... -e NODE_REGION_CODE=DE -e NODE_REGION_CITY="Frankfurt" ...

# Asia Pacific
docker run ... -e NODE_REGION_CODE=JP -e NODE_REGION_CITY="Tokyo" ...
```

## Development vs Production

### Development

- Uses HTTP (no mTLS) for `directory-api` connections
- May use `network_mode: host` for easier debugging
- Uses dev certificates from `tools/pki/dev`

### Production

- **Always enable mTLS**: `MTLS_ENABLED=1`
- Use proper network isolation
- Use production certificates from a secure CA
- Enable firewall rules: `RUN_NFT_SETUP=1`
- Use read-only certificate mounts
- Set up proper logging and monitoring

## Additional Resources

- [README.md](./README.md) - Development and quick start guide
- [DEVELOPMENT.md](./DEVELOPMENT.md) - Development setup and contribution guide
- [WireGuard Documentation](https://www.wireguard.com/)
- [nftables Documentation](https://wiki.nftables.org/)
