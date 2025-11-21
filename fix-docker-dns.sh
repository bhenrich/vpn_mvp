#!/bin/bash
# Script to configure Docker daemon DNS for build-time resolution
# This fixes "Temporary failure resolving 'deb.debian.org'" errors during Docker builds

set -e

DAEMON_JSON="/etc/docker/daemon.json"
DNS_SERVERS='["8.8.8.8", "1.1.1.1"]'

echo "Configuring Docker daemon DNS servers..."

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "Please run as root (use sudo)"
    exit 1
fi

# Backup existing daemon.json if it exists
if [ -f "$DAEMON_JSON" ]; then
    echo "Backing up existing $DAEMON_JSON..."
    cp "$DAEMON_JSON" "${DAEMON_JSON}.backup.$(date +%Y%m%d_%H%M%S)"
    
    # Check if DNS is already configured
    if grep -q '"dns"' "$DAEMON_JSON"; then
        echo "DNS configuration already exists in $DAEMON_JSON"
        echo "Current DNS settings:"
        grep -A 2 '"dns"' "$DAEMON_JSON" || true
        echo ""
        read -p "Do you want to update it? (y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Aborted."
            exit 0
        fi
    fi
fi

# Create or update daemon.json
if [ -f "$DAEMON_JSON" ]; then
    # Use jq if available, otherwise use sed/python
    if command -v jq &> /dev/null; then
        echo "Using jq to update daemon.json..."
        jq ".dns = $DNS_SERVERS" "$DAEMON_JSON" > "${DAEMON_JSON}.tmp" && mv "${DAEMON_JSON}.tmp" "$DAEMON_JSON"
    else
        echo "jq not found, using Python to update daemon.json..."
        python3 << EOF
import json
import sys

try:
    with open('$DAEMON_JSON', 'r') as f:
        config = json.load(f)
except (FileNotFoundError, json.JSONDecodeError):
    config = {}

config['dns'] = $DNS_SERVERS

with open('$DAEMON_JSON', 'w') as f:
    json.dump(config, f, indent=2)
EOF
    fi
else
    echo "Creating new $DAEMON_JSON..."
    cat > "$DAEMON_JSON" << EOF
{
  "dns": $DNS_SERVERS
}
EOF
fi

echo ""
echo "Updated $DAEMON_JSON:"
cat "$DAEMON_JSON"
echo ""

# Restart Docker daemon
echo "Restarting Docker daemon..."
if systemctl is-active --quiet docker; then
    systemctl restart docker
    echo "Docker daemon restarted successfully."
else
    echo "Warning: Docker daemon is not running. Start it with: systemctl start docker"
fi

echo ""
echo "DNS configuration complete!"
echo "You can now rebuild your Docker images."

