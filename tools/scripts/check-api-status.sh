#!/bin/bash
# Check API status and endpoints

echo "=== Checking API Health ==="
echo ""
echo "Auth API:"
curl -s http://localhost:8080/healthz || echo "FAILED"
echo ""
echo "Directory API:"
curl -s http://localhost:8081/healthz || echo "FAILED"
echo ""
echo "Admin API:"
curl -s http://localhost:8082/healthz || echo "FAILED"
echo ""

echo "=== Checking Directory API /regions endpoint ==="
curl -v http://localhost:8081/regions 2>&1
echo ""

echo "=== Checking if containers are running ==="
docker compose ps | grep -E "auth-api|directory-api|admin-api"

echo ""
echo "=== Checking directory-api logs (last 20 lines) ==="
docker compose logs --tail=20 directory-api

echo ""
echo "=== Checking if users exist in database ==="
docker exec vpn_mvp-postgres-1 psql -U postgres -d postgres -c "SELECT id, email, created_at FROM users LIMIT 5;" 2>&1 || echo "Failed to query users"

echo ""
echo "=== Checking if regions exist in database ==="
docker exec vpn_mvp-postgres-1 psql -U postgres -d postgres -c "SELECT id, country_code, city, status FROM regions;" 2>&1 || echo "Failed to query regions"

