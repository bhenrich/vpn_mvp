# Remote Debugging Guide

Since direct SSH access isn't possible, here are several methods to enable remote debugging and troubleshooting on your Linux root server.

## Option 1: Diagnostic Scripts (Recommended)

Create diagnostic scripts that you can run and share the output with me.

### Create Diagnostic Script

```bash
cat > /root/vpn-diagnostics.sh << 'EOF'
#!/bin/bash
# VPN MVP Diagnostic Script

echo "=== VPN MVP Diagnostic Report ==="
echo "Generated: $(date)"
echo ""

echo "=== System Information ==="
uname -a
echo ""

echo "=== Docker Version ==="
docker --version
docker compose version
echo ""

echo "=== Running Containers ==="
docker compose -f docker-compose.prod.yml ps
echo ""

echo "=== Container Logs (Last 50 lines) ==="
echo "--- node-agent-de-berlin ---"
docker logs --tail=50 node-agent-de-berlin 2>&1
echo ""
echo "--- node-agent-de-munich ---"
docker logs --tail=50 node-agent-de-munich 2>&1
echo ""
echo "--- directory-api ---"
docker logs --tail=50 directory-api 2>&1
echo ""

echo "=== WireGuard Status ==="
echo "--- Berlin Node ---"
docker exec node-agent-de-berlin wg show wg0 2>&1 || echo "Error getting wg0 status"
echo ""
echo "--- Munich Node ---"
docker exec node-agent-de-munich wg show wg0 2>&1 || echo "Error getting wg0 status"
echo ""

echo "=== Network Configuration ==="
echo "--- Host Routes ---"
ip route show
echo ""
echo "--- Host Interfaces ---"
ip addr show
echo ""
echo "--- IPv4 Forwarding ---"
sysctl net.ipv4.ip_forward
echo ""

echo "=== nftables Rules ==="
echo "--- Berlin Node ---"
docker exec node-agent-de-berlin nft list table inet vpn 2>&1 || echo "Error getting nftables rules"
echo ""
echo "--- Munich Node ---"
docker exec node-agent-de-munich nft list table inet vpn 2>&1 || echo "Error getting nftables rules"
echo ""

echo "=== Port Listening Status ==="
netstat -tulpn | grep -E "51820|51821|8080|8081|8082|50051"
echo ""

echo "=== Container Resource Usage ==="
docker stats --no-stream
echo ""

echo "=== Disk Usage ==="
df -h
docker system df
echo ""

echo "=== WireGuard Kernel Module ==="
lsmod | grep wireguard || echo "WireGuard module not loaded"
echo ""

echo "=== Recent Container Events ==="
docker events --since 5m --until now 2>&1 | tail -20 || echo "No recent events"
echo ""

echo "=== End of Diagnostic Report ==="
EOF

chmod +x /root/vpn-diagnostics.sh
```

**Usage:**
```bash
# Run diagnostic script
/root/vpn-diagnostics.sh > diagnostics_$(date +%Y%m%d_%H%M%S).txt 2>&1

# Share the output file with me via pastebin, gist, or chat
```

## Option 2: SSH with Jump Host / Port Forwarding

If you can set up a jump host or SSH tunnel, I can help debug through that.

### Setup SSH Tunnel (You Control Access)

```bash
# On your root server, allow SSH access
# Install a temporary SSH key or password-based access

# Create a debug user with limited sudo
sudo useradd -m -s /bin/bash vpn-debug
sudo usermod -aG docker vpn-debug
echo "vpn-debug ALL=(ALL) NOPASSWD: /usr/bin/docker, /usr/bin/docker-compose, /usr/bin/ip, /usr/bin/wg" | sudo tee /etc/sudoers.d/vpn-debug

# Share SSH credentials with me (temporary, remove after debugging)
```

### One-Time SSH Access Script

Create a script that runs specific commands:

```bash
cat > /root/run-remote-command.sh << 'EOF'
#!/bin/bash
# Remote command executor - you run this with the command as argument

CMD="$1"
if [ -z "$CMD" ]; then
    echo "Usage: $0 'command to run'"
    exit 1
fi

# Run command and capture output
eval "$CMD" 2>&1
EOF

chmod +x /root/run-remote-command.sh
```

## Option 3: Web-Based Diagnostic Endpoint

Create a simple HTTP endpoint that runs diagnostics (secure it properly!).

### Simple Flask Diagnostic Service

```bash
cat > /root/diagnostic-api.py << 'EOF'
#!/usr/bin/env python3
from flask import Flask, jsonify
import subprocess
import os

app = Flask(__name__)

# SECURITY: Change this token!
DIAG_TOKEN = "CHANGE_THIS_SECRET_TOKEN"

def run_cmd(cmd):
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=30)
        return {
            "exit_code": result.returncode,
            "stdout": result.stdout,
            "stderr": result.stderr
        }
    except Exception as e:
        return {"error": str(e)}

@app.route('/diagnostics/<token>')
def diagnostics(token):
    if token != DIAG_TOKEN:
        return jsonify({"error": "Invalid token"}), 403
    
    results = {
        "docker_ps": run_cmd("docker compose -f docker-compose.prod.yml ps --format json"),
        "berlin_wg": run_cmd("docker exec node-agent-de-berlin wg show wg0"),
        "munich_wg": run_cmd("docker exec node-agent-de-munich wg show wg0"),
        "berlin_nft": run_cmd("docker exec node-agent-de-berlin nft list table inet vpn"),
        "ip_forward": run_cmd("sysctl net.ipv4.ip_forward"),
        "routes": run_cmd("ip route show"),
    }
    return jsonify(results)

@app.route('/logs/<service>/<token>')
def logs(service, token):
    if token != DIAG_TOKEN:
        return jsonify({"error": "Invalid token"}), 403
    
    result = run_cmd(f"docker logs --tail=100 {service}")
    return jsonify(result)

if __name__ == '__main__':
    app.run(host='127.0.0.1', port=9999, debug=False)
EOF

chmod +x /root/diagnostic-api.py

# Install Flask if needed
pip3 install flask

# Run as systemd service
cat > /etc/systemd/system/vpn-diagnostic-api.service << 'EOF'
[Unit]
Description=VPN Diagnostic API
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/bin/python3 /root/diagnostic-api.py
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable vpn-diagnostic-api
sudo systemctl start vpn-diagnostic-api

# Test (from your server)
curl http://localhost:9999/diagnostics/YOUR_SECRET_TOKEN
```

**Security Note**: This exposes diagnostics over HTTP. For production:
- Use HTTPS
- Change the token regularly
- Restrict access with firewall
- Consider removing after debugging

## Option 4: Live Debugging Session

Set up a screen/tmux session and share access:

```bash
# Install screen or tmux
sudo apt-get install screen tmux

# Create shared debugging session
screen -S vpn-debug
# or
tmux new -s vpn-debug

# Share session (if using screen)
# screen -x vpn-debug
# or give me SSH access temporarily
```

## Option 5: Log Aggregation

Send logs to a service I can access:

### Send to Pastebin/Gist

```bash
cat > /root/send-logs.sh << 'EOF'
#!/bin/bash
# Send logs to pastebin (requires curl and pastebin account)

LOG_FILE="/tmp/vpn-logs-$(date +%Y%m%d-%H%M%S).txt"
docker compose -f docker-compose.prod.yml logs --tail=500 > "$LOG_FILE"
docker exec node-agent-de-berlin wg show wg0 >> "$LOG_FILE" 2>&1
docker exec node-agent-de-berlin nft list table inet vpn >> "$LOG_FILE" 2>&1

# Upload to pastebin (you'll need to get API key from pastebin.com)
# PASTEBIN_API_KEY="your-api-key"
# curl -d "api_option=paste" -d "api_dev_key=$PASTEBIN_API_KEY" -d "api_paste_code=$(cat $LOG_FILE)" -d "api_paste_private=1" http://pastebin.com/api/api_post.php

# Or use gist (requires gh CLI)
# gh gist create "$LOG_FILE" --public

echo "Logs saved to: $LOG_FILE"
EOF

chmod +x /root/send-logs.sh
```

## Recommended Approach

For immediate debugging, I recommend:

1. **Use Option 1 (Diagnostic Scripts)**: Run `/root/vpn-diagnostics.sh` and share the output
2. **Use Option 3 (Diagnostic API)**: If you need ongoing monitoring, set up the Flask endpoint
3. **Share logs directly**: Copy/paste relevant log outputs from `docker logs`

## Quick Debug Commands

When issues arise, run these and share the output:

```bash
# Container status
docker compose -f docker-compose.prod.yml ps

# Recent logs
docker compose -f docker-compose.prod.yml logs --tail=100

# WireGuard status
docker exec node-agent-de-berlin wg show wg0
docker exec node-agent-de-munich wg show wg0

# nftables rules with counters
docker exec node-agent-de-berlin nft list table inet vpn -a

# Network configuration
ip route show
ip addr show
sysctl net.ipv4.ip_forward

# Check if ports are listening
netstat -tulpn | grep -E "51820|51821|8080|8081|8082"
```

## Security Best Practices

1. **Remove temporary access** after debugging
2. **Rotate tokens** regularly if using diagnostic API
3. **Use HTTPS** for any web-based diagnostics
4. **Limit firewall rules** to necessary IPs only
5. **Monitor access logs** for suspicious activity

## Getting Help

When asking for help, include:
1. Output from diagnostic script
2. Relevant log excerpts
3. Description of the issue
4. Steps to reproduce
5. What you've already tried

I can help you interpret the output and suggest fixes even without direct access!

