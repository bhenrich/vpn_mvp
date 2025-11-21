# Production Deployment Quick Checklist

## Critical Changes for Linux Deployment

### ✅ Host Networking for Node Agents (REQUIRED)

**This is the key fix for VPN forwarding!** On Linux, node agents MUST use `network_mode: host`.

**In `docker-compose.prod.yml`:**

```yaml
x-node-agent-base: &node-agent-base
  build: ./nodes/agent
  network_mode: host  # ← ADD THIS (Linux only!)
  cap_add:
    - NET_ADMIN
    - SYS_MODULE
  # ... rest of config
```

**For node agent services:**
- ❌ **REMOVE** `ports:` section (not needed with host networking)
- ✅ Host networking exposes ports directly on the host

### ✅ Replace Placeholder Values

Before deploying, update these in `docker-compose.prod.yml`:

- [ ] `YOUR_SERVER_IP` → Your actual public IP address (appears 4 times)
- [ ] Database passwords in `.env` files (use strong random passwords)
- [ ] JWT secrets (generate with: `openssl rand -hex 32`)
- [ ] Certificate paths (if using mTLS in production)

### ✅ Environment Files

- [ ] Create production `.env` files for each service
- [ ] Update `DATABASE_URL` with strong password
- [ ] Update `REDIS_URL` if needed
- [ ] Set `JWT_SECRET` to a strong random value

### ✅ Firewall Configuration

- [ ] Allow ports 8080, 8081, 8082 (TCP) for APIs
- [ ] Allow ports 51820, 51821 (UDP) for WireGuard
- [ ] Block ports 5432, 6379 (Postgres/Redis - internal only)
- [ ] Ensure SSH (22/TCP) is allowed

### ✅ Pre-Deployment Checks

- [ ] Verify WireGuard kernel module: `modprobe wireguard && lsmod | grep wireguard`
- [ ] Check Docker is installed: `docker --version`
- [ ] Check Docker Compose is installed: `docker compose version`
- [ ] Verify IPv4 forwarding can be enabled: `sysctl net.ipv4.ip_forward`
- [ ] Ensure sufficient disk space: `df -h`

### ✅ Post-Deployment Verification

- [ ] All containers running: `docker compose -f docker-compose.prod.yml ps`
- [ ] Node agents have WireGuard interfaces: `docker exec node-agent-de-berlin wg show wg0`
- [ ] nftables rules applied: `docker exec node-agent-de-berlin nft list table inet vpn`
- [ ] APIs responding: `curl http://localhost:8080/healthz`
- [ ] Test VPN connection from client
- [ ] Verify forwarding works: client can ping `8.8.8.8`

## Deployment Steps (TL;DR)

```bash
# 1. Clone repo
git clone <repo-url>
cd vpn_mvp
git checkout refactor/wireguard-rs

# 2. Configure environment
cp services/*/env.example services/*/.env
# Edit .env files with production values

# 3. Create docker-compose.prod.yml (copy from DEPLOYMENT_PLAN.md)
# IMPORTANT: Add network_mode: host to node-agent-base

# 4. Update YOUR_SERVER_IP in docker-compose.prod.yml (4 places)

# 5. Configure firewall
sudo ufw allow 8080/tcp 8081/tcp 8082/tcp
sudo ufw allow 51820/udp 51821/udp
sudo ufw enable

# 6. Build and deploy
docker compose -f docker-compose.prod.yml build
docker compose -f docker-compose.prod.yml up -d

# 7. Verify
docker compose -f docker-compose.prod.yml ps
docker exec node-agent-de-berlin wg show wg0
```

## Key Differences: Windows Dev vs Linux Production

| Feature | Windows Dev | Linux Production |
|---------|-------------|------------------|
| Network Mode | Bridge (default) | **Host** (required) |
| Port Mappings | Yes (51820:51820/udp) | No (not needed) |
| Forwarding Fix | N/A (doesn't work on Windows Docker) | **Host networking solves it** |
| mTLS | Optional (dev certs) | Recommended (prod certs) |

## Troubleshooting Quick Reference

### Forwarding Not Working?

1. ✅ Verify `network_mode: host` is set
2. ✅ Check `sysctl net.ipv4.ip_forward` (should be 1)
3. ✅ Check nftables rules: `docker exec node-agent-de-berlin nft list table inet vpn`
4. ✅ Check forward chain counters (should increment when client connects)

### Node Agents Won't Start?

1. ✅ Check WireGuard module: `lsmod | grep wireguard`
2. ✅ Check ports aren't in use: `netstat -tulpn | grep 51820`
3. ✅ Check logs: `docker logs node-agent-de-berlin`
4. ✅ Verify capabilities: container needs `NET_ADMIN` and `SYS_MODULE`

### Can't Connect from Client?

1. ✅ Verify firewall allows UDP ports
2. ✅ Check node agent is running: `docker ps | grep node-agent`
3. ✅ Check WireGuard interface: `docker exec node-agent-de-berlin wg show wg0`
4. ✅ Test from client: `ping 8.8.8.8` (should work after connecting)

## Security Checklist

- [ ] Changed all default passwords
- [ ] Generated strong JWT secrets
- [ ] Using production certificates (if mTLS enabled)
- [ ] Firewall configured properly
- [ ] Database ports not exposed externally
- [ ] Regular backups configured
- [ ] Log monitoring set up

## Need Help?

1. Run diagnostic script: `/root/vpn-diagnostics.sh` (see REMOTE_DEBUGGING.md)
2. Share output with me
3. Check logs: `docker compose -f docker-compose.prod.yml logs`
4. Review DEPLOYMENT_PLAN.md for detailed steps

