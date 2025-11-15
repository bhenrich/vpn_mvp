#!/bin/bash
# Manually create regions via API (requires authentication token)

AUTH_URL="http://localhost:8080"
DIRECTORY_URL="http://localhost:8081"

# Default credentials (change if needed)
EMAIL="admin@test.de"
PASSWORD="test"

echo "=== Authenticating ==="
TOKEN=$(curl -s -X POST "${AUTH_URL}/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}" | \
  jq -r '.access_token')

if [ -z "$TOKEN" ] || [ "$TOKEN" = "null" ]; then
    echo "ERROR: Failed to authenticate. Please create an admin user first."
    exit 1
fi

echo "Got token: ${TOKEN:0:20}..."

echo ""
echo "=== Creating Berlin Region ==="
curl -X POST "${DIRECTORY_URL}/regions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${TOKEN}" \
  -d '{"country_code":"DE","city":"Berlin","status":"active"}' | jq '.'

echo ""
echo "=== Creating Munich Region ==="
curl -X POST "${DIRECTORY_URL}/regions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${TOKEN}" \
  -d '{"country_code":"DE","city":"Munich","status":"active"}' | jq '.'

echo ""
echo "=== Verifying Regions ==="
curl -s "${DIRECTORY_URL}/regions" | jq '.'

