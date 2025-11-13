# auth-api (FastAPI skeleton)

Authentication and user management service (WS2).

Quick start:

1. Copy environment template:

   ```bash
   cp env.example .env
   ```

2. Build and run via compose from repo root:

   ```bash
   docker compose up --build auth-api
   ```

3. Health check:

   ```bash
   curl http://localhost:8080/healthz
   ```

Endpoints (dev):

- JWKS:

  ```bash
  curl http://localhost:8080/.well-known/jwks.json
  ```

- Register user:

  ```bash
  curl -X POST http://localhost:8080/users/register \
    -H "Content-Type: application/json" \
    -d '{"email":"alice@example.com","password":"CorrectHorseBatteryStaple","consent":true,"tos_version":"v1"}'
  ```

- Login (returns access and refresh tokens):

  ```bash
  curl -X POST http://localhost:8080/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email":"alice@example.com","password":"CorrectHorseBatteryStaple"}'
  ```

- Refresh:

  ```bash
  curl -X POST http://localhost:8080/auth/refresh \
    -H "Content-Type: application/json" \
    -d '{"refresh_token":"<JWT>"}'
  ```

- Register device (requires Bearer access token):

  ```bash
  curl -X POST http://localhost:8080/devices/register \
    -H "Authorization: Bearer <ACCESS_TOKEN>" \
    -H "Content-Type: application/json" \
    -d '{"device_public_key_b64":"<BASE64_PUBLIC_KEY>","platform":"windows"}'
  ```

- PKCE device login (device flow):

  1) Start:

  ```bash
  curl -X POST http://localhost:8080/auth/device/start \
    -H "Content-Type: application/json" \
    -d '{"code_challenge":"<CHALLENGE>","code_challenge_method":"S256","user_hint":"alice@example.com"}'
  ```

  2) Authorize (simulate user login approving device):

  ```bash
  curl -X POST http://localhost:8080/auth/device/authorize \
    -H "Content-Type: application/json" \
    -d '{"device_code":"<DEVICE_CODE>","email":"alice@example.com","password":"CorrectHorseBatteryStaple"}'
  ```

  3) Poll from device with code_verifier:

  ```bash
  curl -X POST http://localhost:8080/auth/device/poll \
    -H "Content-Type: application/json" \
    -d '{"device_code":"<DEVICE_CODE>","code_verifier":"<VERIFIER>"}'
  ```


