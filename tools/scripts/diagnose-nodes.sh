#!/bin/bash
# Diagnostic script to check regions and node registration

echo "=== Checking Regions ==="
curl -s http://localhost:8081/regions | jq '.' || echo "Failed to get regions"

echo ""
echo "=== Checking Node Agent Logs ==="
echo "--- Berlin Node ---"
docker compose logs --tail=50 node-agent-de-berlin | grep -i -E "register|error|region|fatal" || docker compose logs --tail=20 node-agent-de-berlin

echo ""
echo "--- Munich Node ---"
docker compose logs --tail=50 node-agent-de-munich | grep -i -E "register|error|region|fatal" || docker compose logs --tail=20 node-agent-de-munich

echo ""
echo "=== Checking Node Agent Status ==="
docker compose ps | grep node-agent

echo ""
echo "=== Checking Registered Nodes ==="
# This endpoint might require auth, but let's try
curl -s http://localhost:8081/nodes 2>&1 | head -20 || echo "Failed to get nodes (might require auth)"

