# PowerShell helper script to create a test user for end-to-end testing
# Usage: .\create-test-user.ps1 -Email "test@example.com" -Password "testpass123"

param(
    [string]$Email = "test@example.com",
    [string]$Password = "testpass123",
    [string]$AuthApiUrl = "http://localhost:8080"
)

Write-Host "Creating test user: $Email" -ForegroundColor Cyan
Write-Host "Using auth-api at: $AuthApiUrl" -ForegroundColor Gray

try {
    # Try registration endpoint (auth-api uses /users/register)
    $body = @{
        email = $Email
        password = $Password
    } | ConvertTo-Json

    $response = Invoke-RestMethod -Uri "$AuthApiUrl/users/register" `
        -Method Post `
        -ContentType "application/json" `
        -Body $body `
        -ErrorAction Stop

    Write-Host "User created successfully!" -ForegroundColor Green
    Write-Host "Email: $Email" -ForegroundColor White
    Write-Host "Password: $Password" -ForegroundColor White
    Write-Host "User ID: $($response.id)" -ForegroundColor Gray
    exit 0
}
catch {
    Write-Host "Registration endpoint not available or failed." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Alternative: Create user directly in database:" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "docker compose exec postgres psql -U postgres -d postgres -c `"INSERT INTO users (id, email, password_hash, created_at, updated_at) VALUES (gen_random_uuid(), '$Email', '\$2b\$12\$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqJ5q5q5q5', NOW(), NOW()) ON CONFLICT (email) DO NOTHING;`"" -ForegroundColor White
    Write-Host ""
    Write-Host "Note: The password hash is a placeholder. For production, use proper bcrypt hashing." -ForegroundColor Yellow
    exit 1
}

