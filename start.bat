@echo off
setlocal

REM check if any containers are running and stop them
docker compose down
docker compose rm -f

REM Ensure env file exists; use server\.env for compose
if not exist server\.env (
  echo server\.env not found. Creating default...
  >server\.env echo POSTGRES_USER=postgres
  >>server\.env echo POSTGRES_PASSWORD=postgres
  >>server\.env echo POSTGRES_DB=vpn
  >>server\.env echo POSTGRES_DSN=postgres://postgres:postgres@postgres:5432/vpn?sslmode=disable
  >>server\.env echo JWT_SIGNING_KEY=dev-change-me
  >>server\.env echo OVPN_REMOTE=openvpn
  >>server\.env echo OVPN_PORT=1194
  >>server\.env echo OVPN_PROTO=udp
  >>server\.env echo TEST_USER_EMAIL=test
  >>server\.env echo TEST_USER_PASSWORD=test
)

REM Bring up all containers
echo Starting docker services...
docker compose --env-file server\.env up -d --build
if errorlevel 1 (
  echo Docker compose failed.
  goto :eof
)

REM Wait for auth service to be reachable
echo Waiting for auth service TCP port 8080 ...
powershell -NoProfile -Command "$deadline=(Get-Date).AddSeconds(60); while((Get-Date) -lt $deadline){ try { $c = New-Object System.Net.Sockets.TcpClient; $iar = $c.BeginConnect('localhost',8080,$null,$null); if($iar.AsyncWaitHandle.WaitOne(2000) -and $c.Connected){ $c.Close(); exit 0 } $c.Close() } catch {} Start-Sleep -Seconds 1 }; exit 1"
if errorlevel 1 (
  echo Auth service did not become ready. Check 'docker compose logs auth'.
  goto :eof
)

REM Set client env vars for this session
set AUTH_BASE_URL=http://localhost:8080

REM Build client
echo Building client...
dotnet build client\VpnClient.sln -c Debug
if errorlevel 1 (
  echo Build failed.
  goto :eof
)

REM Run client WinForms app
set CLIENT_EXE=client\App\bin\Debug\net8.0-windows\App.exe
if not exist "%CLIENT_EXE%" (
  echo Client executable not found at %CLIENT_EXE%.
  goto :eof
)

echo Launching client...
start "VPN Client" "%CLIENT_EXE%"

echo Done.
endlocal
