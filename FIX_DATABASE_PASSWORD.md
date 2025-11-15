# Fix Database Password Mismatch

## The Problem

The `directory-api` container is failing because the PostgreSQL password in `.env` doesn't match the password set for the PostgreSQL container.

## Solution 1: Use Default Password (Quick Fix)

If you want to use the default password `example`:

```bash
# Create/update directory-api/.env
cat > services/directory-api/.env << 'EOF'
PORT=8081
GRPC_PORT=50051
DATABASE_URL=postgresql://postgres:example@postgres:5432/postgres
REDIS_URL=redis://redis:6379/0
GRPC_TLS_ENABLED=0
GRPC_TLS_CERT_PATH=/certs/directory.crt
GRPC_TLS_KEY_PATH=/certs/directory.key
GRPC_TLS_CLIENT_CA_PATH=/certs/ca.crt
EOF

# Restart directory-api
docker compose restart directory-api
```

## Solution 2: Use Custom Password (Recommended for Production)

Set a strong password and ensure it matches in all places:

```bash
# Generate a strong password
STRONG_PASSWORD=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-25)
echo "Generated password: $STRONG_PASSWORD"

# Set it in docker-compose.yml environment variable
export POSTGRES_PASSWORD="$STRONG_PASSWORD"

# Update directory-api/.env
cat > services/directory-api/.env << EOF
PORT=8081
GRPC_PORT=50051
DATABASE_URL=postgresql://postgres:${STRONG_PASSWORD}@postgres:5432/postgres
REDIS_URL=redis://redis:6379/0
GRPC_TLS_ENABLED=0
GRPC_TLS_CERT_PATH=/certs/directory.crt
GRPC_TLS_KEY_PATH=/certs/directory.key
GRPC_TLS_CLIENT_CA_PATH=/certs/ca.crt
EOF

# Also update auth-api/.env (if it exists)
cat > services/auth-api/.env << EOF
DATABASE_URL=postgresql://postgres:${STRONG_PASSWORD}@postgres:5432/postgres
REDIS_URL=redis://redis:6379/0
# ... other variables ...
EOF

# Also update admin-api/.env (if it exists)
cat > services/admin-api/.env << EOF
DATABASE_URL=postgresql://postgres:${STRONG_PASSWORD}@postgres:5432/postgres
# ... other variables ...
EOF

# Stop and remove postgres container to recreate with new password
docker compose stop postgres
docker compose rm -f postgres
docker volume rm vpn_mvp_postgres-data  # WARNING: This deletes existing data!

# Start with new password
docker compose up -d postgres

# Wait for postgres to be healthy
docker compose ps postgres

# Restart all services
docker compose up -d
```

## Solution 3: Check Existing Password

If postgres container already exists with a password, check what it's using:

```bash
# Check postgres container environment
docker inspect vpn_mvp-postgres-1 | grep POSTGRES_PASSWORD

# Or check docker-compose ps output
docker compose config | grep POSTGRES_PASSWORD
```

Then update your `.env` files to match.

## Verification

After fixing:

```bash
# Check directory-api logs
docker compose logs directory-api

# Should see successful database connection
# Look for lines like "Database initialized" or similar

# Test the API
curl http://localhost:8081/healthz
```

## Important Notes

- The password in `DATABASE_URL` must match `POSTGRES_PASSWORD` environment variable
- If you change the password, you need to update ALL `.env` files that reference the database:
  - `services/auth-api/.env`
  - `services/directory-api/.env`
  - `services/admin-api/.env`
- Changing the password after postgres is initialized requires recreating the postgres container and volume (data loss!)

