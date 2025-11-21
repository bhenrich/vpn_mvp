#!/bin/bash
# Helper script to create a test user for end-to-end testing
# Usage: ./create-test-user.sh <email> <password>

set -e

EMAIL="${1:-test@example.com}"
PASSWORD="${2:-testpass123}"
AUTH_API_URL="${AUTH_API_URL:-http://localhost:8080}"

echo "Creating test user: $EMAIL"
echo "Using auth-api at: $AUTH_API_URL"

# Check if registration endpoint exists (auth-api uses /users/register)
if curl -s -f "${AUTH_API_URL}/users/register" > /dev/null 2>&1; then
    echo "Using registration endpoint..."
    RESPONSE=$(curl -s -X POST "${AUTH_API_URL}/users/register" \
        -H "Content-Type: application/json" \
        -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}")
    
    if echo "$RESPONSE" | grep -q "error\|Error\|failed"; then
        echo "Registration failed: $RESPONSE"
        exit 1
    else
        echo "User created successfully!"
        echo "Email: $EMAIL"
        echo "Password: $PASSWORD"
        exit 0
    fi
else
    echo "Registration endpoint not available. Creating user directly in database..."
    echo ""
    echo "Run this SQL command in your postgres container:"
    echo ""
    echo "docker compose exec postgres psql -U postgres -d postgres -c \\"
    echo "  \"INSERT INTO users (id, email, password_hash, created_at, updated_at)"
    echo "   VALUES ("
    echo "     gen_random_uuid(),"
    echo "     '${EMAIL}',"
    echo "     '\\\$2b\\\$12\\\$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqJ5q5q5q5',"
    echo "     NOW(),"
    echo "     NOW()"
    echo "   ) ON CONFLICT (email) DO NOTHING;\""
    echo ""
    echo "Note: The password hash above is a placeholder."
    echo "For production, use the auth-api registration endpoint or generate a proper bcrypt hash."
    exit 1
fi

