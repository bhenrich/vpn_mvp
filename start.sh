#!/usr/bin/bash

set -euo pipefail

# Check if Docker is installed
if ! command -v docker &>/dev/null; then
  echo "Docker is not installed. Please install Docker first."
  exit 1
fi

# Shut down any running containers from this project and remove stopped containers
docker compose down
docker compose rm -f

# Ensure env file exists; use server/.env for compose
if [[ ! -f server/.env ]]; then
  echo "server/.env not found. Creating default..."
  cat > server/.env << EOF
POSTGRES_USER=postgres
POSTGRES_PASSWORD=postgres
POSTGRES_DB=vpn
POSTGRES_DSN=postgres://postgres:postgres@postgres:5432/vpn?sslmode=disable
JWT_SIGNING_KEY=dev-change-me
OVPN_REMOTE=openvpn
OVPN_PORT=1194
OVPN_PROTO=udp
TEST_USER_EMAIL=test
TEST_USER_PASSWORD=test
EOF
fi

# Bring up all containers
echo "Starting docker services..."
if ! docker compose --env-file server/.env up -d --build; then
  echo "Docker compose failed."
  exit 1
fi

# Wait for auth service to be reachable on TCP port 8080, up to 60 seconds
echo "Waiting for auth service TCP port 8080 ..."
success=false
for i in {1..60}; do
  if nc -z localhost 8080; then
    success=true
    break
  fi
  sleep 1
done

if ! $success; then
  echo "Auth service did not become ready. Check 'docker compose logs auth'."
  exit 1
fi

# Set client env vars for this session
export AUTH_BASE_URL="http://localhost:8080"

# Build client
echo "Building client..."
if ! dotnet build client/VpnClient.sln -c Debug; then
  echo "Build failed."
  exit 1
fi

# Run client WinForms app
CLIENT_EXE="client/App/bin/Debug/net8.0-windows/App.exe"
if [[ ! -f "$CLIENT_EXE" ]]; then
  echo "Client executable not found at $CLIENT_EXE."
  exit 1
fi

echo "Launching client..."
wine "$CLIENT_EXE" &

echo "Done."
