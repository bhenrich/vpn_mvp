#!/bin/bash
# Manually create regions via API (requires authentication token)

AUTH_URL="http://localhost:8080"
DIRECTORY_URL="http://localhost:8081"

# Default credentials (change if needed)
EMAIL="admin@test.de"
PASSWORD="test"

echo "=== Authenticating ==="
AUTH_RESPONSE=$(curl -s -X POST "${AUTH_URL}/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}")

echo "Auth response: $AUTH_RESPONSE"

# Extract token using grep and sed (no jq required)
TOKEN=$(echo "$AUTH_RESPONSE" | grep -o '"access_token":"[^"]*' | sed 's/"access_token":"//')

if [ -z "$TOKEN" ]; then
    echo ""
    echo "ERROR: Failed to authenticate."
    echo "Auth response was: $AUTH_RESPONSE"
    echo ""
    echo "This might mean:"
    echo "1. The user doesn't exist yet - need to create it first"
    echo "2. Wrong credentials"
    echo ""
    echo "Let's try creating the user first..."
    echo ""
    
    # Try to register the user first
    REGISTER_RESPONSE=$(curl -s -X POST "${AUTH_URL}/auth/register" \
      -H "Content-Type: application/json" \
      -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\",\"name\":\"Admin User\"}")
    
    echo "Registration response: $REGISTER_RESPONSE"
    
    # Try to login again
    AUTH_RESPONSE=$(curl -s -X POST "${AUTH_URL}/auth/login" \
      -H "Content-Type: application/json" \
      -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}")
    
    TOKEN=$(echo "$AUTH_RESPONSE" | grep -o '"access_token":"[^"]*' | sed 's/"access_token":"//')
    
    if [ -z "$TOKEN" ]; then
        echo "Still failed to authenticate. Response: $AUTH_RESPONSE"
        exit 1
    fi
fi

echo "Got token: ${TOKEN:0:30}..."

echo ""
echo "=== Creating Berlin Region ==="
curl -X POST "${DIRECTORY_URL}/regions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${TOKEN}" \
  -d '{"country_code":"DE","city":"Berlin","status":"active"}'

echo ""
echo ""
echo "=== Creating Munich Region ==="
curl -X POST "${DIRECTORY_URL}/regions" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${TOKEN}" \
  -d '{"country_code":"DE","city":"Munich","status":"active"}'

echo ""
echo ""
echo "=== Verifying Regions ==="
curl -s "${DIRECTORY_URL}/regions"
echo ""

