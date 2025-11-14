# start.ps1

# Shut down and remove any running containers
Write-Host "Stopping and removing containers..."
docker compose down --remove-orphans

# Modern Docker Compose automatically loads .env, but ensure it exists
if (-not (Test-Path ".env")) {
    Write-Host ".env not found. Creating default..."
    @"
POSTGRES_PASSWORD=example
POSTGRES_USER=postgres
POSTGRES_DB=postgres
GRPC_TLS_ENABLED=0
GRPC_TLS_CERT_PATH=/certs/directory.crt
GRPC_TLS_KEY_PATH=/certs/directory.key
GRPC_TLS_CLIENT_CA_PATH=/certs/ca.crt
"@ | Set-Content ".env"
}

# Start all services using modern Docker Compose
Write-Host "Starting docker compose services..."
docker compose up -d --build --wait
if ($LASTEXITCODE -ne 0) {
    Write-Host "Docker compose failed."
    exit 1
}

# Wait for the directory-api to be reachable on port 8081
$SERVICE = "directory-api"
$PORT = 8081
Write-Host "Waiting for $SERVICE to become reachable on port $PORT ..."

$TIMEOUT = 60
$ELAPSED = 0

while ($ELAPSED -lt $TIMEOUT) {
    $client = New-Object System.Net.Sockets.TcpClient
    try {
        $client.Connect("127.0.0.1", $PORT)
    } catch {}
    if ($client.Connected) {
        $client.Close()
        break
    }
    Start-Sleep -Seconds 1
    $ELAPSED++
    if ($ELAPSED -ge $TIMEOUT) {
        Write-Host "Timeout waiting for $SERVICE on port $PORT."
        exit 1
    }
}

Write-Host "$SERVICE is ready!"

# Run the seeding script
Write-Host "Running seeding script for demo nodes..."
python services/directory-api/scripts/seed_demo_nodes.py --base-url http://127.0.0.1:8081 --email admin@test.de --password test

Write-Host "All done."
