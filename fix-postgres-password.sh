#!/bin/bash
# Fix PostgreSQL password mismatch

echo "=== Fixing PostgreSQL Password Mismatch ==="
echo ""

# Check what POSTGRES_PASSWORD is set in docker-compose
echo "1. Checking docker-compose configuration..."
docker compose config | grep -A 2 POSTGRES_PASSWORD || echo "POSTGRES_PASSWORD not found in config"
echo ""

# Check environment variable
echo "2. Checking POSTGRES_PASSWORD environment variable..."
echo "POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-not set (will default to 'example')}"
echo ""

echo "3. Stopping postgres container..."
docker compose stop postgres

echo "4. Removing postgres container..."
docker compose rm -f postgres

echo "5. Removing postgres volume (THIS WILL DELETE ALL DATA)..."
read -p "Are you sure you want to delete the postgres data? (yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    echo "Aborted. To fix without data loss, you'll need to manually reset the password in PostgreSQL."
    exit 1
fi

docker volume rm vpn_mvp_postgres-data 2>/dev/null || echo "Volume already removed or doesn't exist"

echo "6. Setting POSTGRES_PASSWORD environment variable..."
export POSTGRES_PASSWORD=example

echo "7. Starting postgres with correct password..."
docker compose up -d postgres

echo "8. Waiting for postgres to be healthy..."
sleep 10

echo "9. Checking postgres logs..."
docker compose logs --tail=20 postgres

echo ""
echo "=== Done! ==="
echo "PostgreSQL should now be initialized with password: example"
echo "Make sure your .env files use: postgresql://postgres:example@postgres:5432/postgres"

