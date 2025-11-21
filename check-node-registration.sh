#!/bin/bash
# Check node agent status and registration

echo "=== Checking Node Agent Container Status ==="
docker compose ps | grep node-agent

echo ""
echo "=== Checking Node Agent Logs (Berlin) ==="
docker compose logs --tail=50 node-agent-de-berlin | grep -i -E "register|error|fatal|region|connected|started|running" || docker compose logs --tail=30 node-agent-de-berlin

echo ""
echo "=== Checking Node Agent Logs (Munich) ==="
docker compose logs --tail=50 node-agent-de-munich | grep -i -E "register|error|fatal|region|connected|started|running" || docker compose logs --tail=30 node-agent-de-munich

echo ""
echo "=== Checking if nodes are registered in database ==="
docker exec vpn_mvp-postgres-1 psql -U postgres -d postgres -c "SELECT id, region_id, public_key, public_endpoint, listen_port, status FROM nodes;" 2>&1

echo ""
echo "=== Checking WireGuard interfaces in node containers ==="
echo "Berlin:"
docker exec node-agent-de-berlin wg show wg0 2>&1 || echo "No wg0 interface or container not running"

echo ""
echo "Munich:"
docker exec node-agent-de-munich wg show wg0 2>&1 || echo "No wg0 interface or container not running"

echo ""
echo "=== Checking node agent environment variables ==="
echo "Berlin environment:"
docker exec node-agent-de-berlin env | grep -E "NODE_REGION|PUBLIC_ENDPOINT|DIRECTORY" | sort

echo ""
echo "Munich environment:"
docker exec node-agent-de-munich env | grep -E "NODE_REGION|PUBLIC_ENDPOINT|DIRECTORY" | sort

