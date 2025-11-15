# End-to-End Testing Guide

This guide walks through testing the VPN system with two nodes running on a Windows PC and a Linux laptop as the client.

## Quick Start Checklist

- [ ] Backend services running on Windows PC
- [ ] Two nodes (de-berlin, de-munich) registered and online
- [ ] Test user account created
- [ ] Linux laptop hosts file configured
- [ ] Tauri client built and running
- [ ] Client can login and see regions
- [ ] Client can connect to VPN
- [ ] Traffic routes through VPN
- [ ] Client can disconnect cleanly

## Architecture Overview

```
┌─────────────────┐         ┌──────────────────┐
│  Linux Laptop   │         │   Windows PC     │
│   (Client)      │────────▶│   (Server)       │
│                 │         │                  │
│  Tauri App      │         │  ┌────────────┐  │
│  WireGuard      │         │  │ auth-api   │  │
│                 │         │  │ directory  │  │
│                 │         │  │ -api       │  │
│                 │         │  └────────────┘  │
│                 │         │                  │
│                 │         │  ┌────────────┐  │
│                 │         │  │ node-agent  │  │
│                 │────────▶│  │ -de-berlin │  │
│                 │         │  └────────────┘  │
│                 │         │                  │
│                 │         │  ┌────────────┐  │
│                 │         │  │ node-agent │  │
│                 │────────▶│  │ -de-munich │  │
│                 │         │  └────────────┘  │
└─────────────────┘         └──────────────────┘
```

## Prerequisites

### On Windows PC (Server)
- Docker Desktop installed and running
- Windows Firewall disabled (for testing)
- Note the PC's LAN IP address (e.g., `192.168.0.183`)

### On Linux Laptop (Client)
- Tauri client built and ready
- WireGuard tools installed: `sudo pacman -S wireguard-tools` (Arch) or equivalent
- Network access to the Windows PC

## Step 1: Start Backend Services on Windows PC

From the repository root on your Windows PC:

```powershell
# Start all backend services and both nodes
docker compose up -d postgres redis auth-api directory-api admin-api directory-seed `
  node-agent-de-berlin node-agent-de-munich

# Wait a few seconds for services to start, then check logs
docker compose logs --tail=50 directory-api
docker compose logs --tail=50 node-agent-de-berlin
docker compose logs --tail=50 node-agent-de-munich
```

**Expected output:**
- `directory-api` should show "Application startup complete"
- `node-agent-de-berlin` should show "registered node for region ... with endpoint de-berlin.dev.local:51820"
- `node-agent-de-munich` should show "registered node for region ... with endpoint de-munich.dev.local:51821"

**Verify nodes are registered:**
```powershell
# Check that nodes appear in the directory
curl http://localhost:8081/regions
```

You should see regions with nodes listed.

## Step 2: Configure Linux Laptop Hosts File

On your Linux laptop, add hostname mappings to point to your Windows PC's IP:

```bash
# Edit hosts file (requires sudo)
sudo nano /etc/hosts

# Add these lines (replace 192.168.0.183 with your PC's actual IP):
192.168.0.183   de-berlin.dev.local
192.168.0.183   de-munich.dev.local
```

**Verify DNS resolution:**
```bash
ping -c 2 de-berlin.dev.local
ping -c 2 de-munich.dev.local
```

Both should resolve to your PC's IP address.

## Step 3: Create Test User Account

On your Windows PC, create a test user using the helper script:

```powershell
# From repository root
cd tools
.\create-test-user.ps1 -Email "test@example.com" -Password "testpass123"
```

**Alternative - Manual creation:**

If the helper script doesn't work, you can create a user directly in the database. First, generate a proper password hash:

```powershell
# Using Python to generate bcrypt hash
docker compose exec auth-api python -c "
from passlib.context import CryptContext
pwd_context = CryptContext(schemes=['bcrypt'], deprecated='auto')
print(pwd_context.hash('testpass123'))
"
```

Then insert the user:

```powershell
# Replace HASH_HERE with the hash from above
docker compose exec postgres psql -U postgres -d postgres -c "
INSERT INTO users (id, email, password_hash, created_at, updated_at)
VALUES (
  gen_random_uuid(),
  'test@example.com',
  'HASH_HERE',
  NOW(),
  NOW()
) ON CONFLICT (email) DO NOTHING;
"
```

**Verify user was created:**
```powershell
docker compose exec postgres psql -U postgres -d postgres -c "SELECT email, created_at FROM users WHERE email = 'test@example.com';"
```

## Step 4: Build and Launch Tauri Client on Linux Laptop

On your Linux laptop:

```bash
cd clients/desktop/ui

# Install dependencies (if not already done)
pnpm install

# Build and launch in dev mode
pnpm tauri:dev
```

The Tauri app window should open.

## Step 5: Configure Client to Connect to PC

In the Tauri app:

1. **First-time setup:**
   - Review and accept telemetry consent if prompted
   - Click "Continue"

2. **Configure server address:**
   - In the "Server address" field, enter your Windows PC's IP (e.g., `192.168.0.183`)
   - Click "Save server"
   - You should see a green success message

3. **Login:**
   - Enter your test email: `test@example.com`
   - Enter your test password: `testpass123`
   - Click "Sign in"
   - If successful, you'll see the Regions selection screen

## Step 6: Select Region and Connect

1. **Select a region:**
   - Choose either "Germany • Berlin" or "Germany • Munich" from the dropdown
   - Select connection mode: "Single-server" or "Multi-hop"
   - Click "Save"
   - You should be taken to the Status screen

2. **Connect to VPN:**
   - Click the "Connect" button
   - You should see:
     - A "Loading..." state on the button
     - Then a green success message: "Successfully connected to VPN"
     - Connection status should show "Connected" in green

## Step 7: Verify VPN Connection

On your Linux laptop, verify the connection:

```bash
# Check WireGuard interface is up
sudo wg show

# You should see an interface like wg-xxxxxxxx with:
# - A peer (the node's public key)
# - Allowed IPs (likely 0.0.0.0/0 for full tunnel)
# - Endpoint pointing to de-berlin.dev.local:51820 or de-munich.dev.local:51821

# Check interface IP
ip addr show | grep wg

# Ping the node's internal IP
ping -c 3 10.66.0.1

# Test DNS resolution through VPN
nslookup google.com 10.66.0.1

# Check routing (should show traffic going through wg interface)
ip route show

# Test internet connectivity through VPN
curl -4 https://ifconfig.co
# This should show your PC's public IP (or the node's egress IP if configured)
```

## Step 8: Test Traffic Flow

### Test 1: Internal VPN Network
```bash
# Ping the node's WireGuard interface
ping -c 5 10.66.0.1

# Should see successful pings
```

### Test 2: DNS Resolution
```bash
# Query DNS through VPN
dig @10.66.0.1 google.com

# Or using nslookup
nslookup google.com 10.66.0.1
```

### Test 3: Internet Traffic
```bash
# Check your public IP (should route through VPN)
curl -4 https://ifconfig.co

# Test HTTP connectivity
curl -4 https://www.google.com
```

### Test 4: Verify Traffic is Routed
```bash
# Monitor traffic on WireGuard interface
sudo tcpdump -i wg-* -n

# In another terminal, generate some traffic
curl https://www.google.com

# You should see packets on the WireGuard interface
```

## Step 9: Test Disconnection

1. **In the Tauri app:**
   - Click the "Disconnect" button
   - You should see a success message: "Disconnected from VPN"
   - Connection status should show "Disconnected" in gray

2. **On Linux laptop:**
   ```bash
   # Verify WireGuard interface is down
   sudo wg show
   # Should show no interfaces or the interface should be down
   
   # Verify routing is back to normal
   ip route show
   # Default route should be back to your normal gateway
   ```

## Step 10: Test Second Node

1. **Disconnect from current node** (if connected)

2. **In the Tauri app:**
   - Go back to Regions (or logout and login again)
   - Select the other region (if you were on Berlin, select Munich, or vice versa)
   - Click "Save"
   - Click "Connect"

3. **Verify connection to second node:**
   ```bash
   sudo wg show
   # Endpoint should now point to the other node (de-munich.dev.local:51821 or de-berlin.dev.local:51820)
   
   ping -c 3 10.66.0.1
   # Should still work
   ```

## Step 11: Check Node Logs

On your Windows PC, monitor the node logs:

```powershell
# Watch Berlin node logs
docker compose logs -f node-agent-de-berlin

# In another terminal, watch Munich node logs
docker compose logs -f node-agent-de-munich
```

**What to look for:**
- Peer additions when client connects
- Heartbeat messages every 30 seconds
- Config stream updates
- Any error messages

## Step 12: Test Multi-Hop Mode

1. **In the Tauri app:**
   - Disconnect if connected
   - Go to Regions
   - Select a region
   - Change mode to "Multi-hop"
   - Click "Save"
   - Click "Connect"

2. **Verify multi-hop configuration:**
   ```bash
   sudo wg show
   # Should show two peers (entry and exit nodes)
   
   # Check routing
   ip route show
   ```

## Troubleshooting

### Client Can't Connect to Server

**Symptoms:** "Login failed" or "Failed to fetch regions"

**Solutions:**
```bash
# On Linux laptop, verify connectivity to PC
ping 192.168.0.183  # Replace with your PC's IP

# Test API endpoints
curl http://192.168.0.183:8080/healthz  # auth-api
curl http://192.168.0.183:8081/healthz  # directory-api

# Check if ports are accessible
nc -zv 192.168.0.183 8080
nc -zv 192.168.0.183 8081
```

### Nodes Not Registering

**Symptoms:** No regions appear in client, or nodes show as offline

**Solutions:**
```powershell
# Check node logs
docker compose logs node-agent-de-berlin
docker compose logs node-agent-de-munich

# Verify directory-api is accessible from nodes
docker compose exec node-agent-de-berlin curl http://directory-api:8081/healthz

# Check if regions exist
curl http://localhost:8081/regions
```

### DNS Resolution Fails

**Symptoms:** Can't resolve hostnames, but ping to 10.66.0.1 works

**Solutions:**
```bash
# Check DNS server is running in node
docker compose exec node-agent-de-berlin unbound-control status

# Test DNS directly
dig @10.66.0.1 google.com

# Check if DNS is configured in WireGuard
sudo wg show | grep -A 5 "allowed ips"
```

### Connection Drops Immediately

**Symptoms:** Connects but immediately disconnects

**Solutions:**
```bash
# Check WireGuard interface logs
sudo dmesg | grep -i wireguard

# Verify node has peer configured
docker compose exec node-agent-de-berlin wg show

# Check node logs for peer removal
docker compose logs node-agent-de-berlin | grep -i peer
```

### Traffic Not Routing Through VPN

**Symptoms:** Connected but traffic goes through normal gateway

**Solutions:**
```bash
# Check routing table
ip route show

# Verify default route points to WireGuard interface
# If not, check kill switch configuration

# Check WireGuard allowed IPs
sudo wg show | grep "allowed ips"
# Should include 0.0.0.0/0 for full tunnel
```

## Expected Test Results

### Successful Test Indicators

✅ **Connection:**
- Client shows "Connected" in green
- WireGuard interface is up with peer configured
- Can ping 10.66.0.1
- DNS resolution works through 10.66.0.1

✅ **Traffic Routing:**
- Internet traffic routes through VPN
- Public IP check shows node's egress IP (or PC's IP)
- All HTTP/HTTPS requests work

✅ **Node Health:**
- Nodes show as "online" in directory-api
- Heartbeats appear in logs every 30 seconds
- Peer configurations are applied

✅ **Disconnection:**
- Clean disconnect removes peer from node
- Routing returns to normal
- WireGuard interface is removed

### Performance Benchmarks

- **Connection time:** < 3 seconds from click to connected
- **DNS resolution:** < 100ms for cached, < 500ms for uncached
- **Latency:** < 50ms to node (on same LAN)
- **Throughput:** Should match your LAN speed (limited by WireGuard overhead ~5-10%)

## Cleanup

After testing:

```powershell
# Stop all services
docker compose down

# Remove volumes (optional - clears database)
docker compose down -v
```

```bash
# On Linux laptop, remove hosts file entries (optional)
sudo nano /etc/hosts
# Remove the de-berlin.dev.local and de-munich.dev.local lines
```

## Next Steps

Once basic testing passes:

1. **Test with Windows Firewall enabled** (configure proper rules)
2. **Test across different networks** (not just same LAN)
3. **Test with real public IPs** (not just dev.local hostnames)
4. **Load testing** (multiple clients connecting simultaneously)
5. **Failover testing** (disconnect one node, verify client reconnects)

