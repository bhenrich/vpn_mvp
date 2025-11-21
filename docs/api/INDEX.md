# API Documentation

This section provides comprehensive documentation for all API contracts, schemas, and integration patterns used across the VPN system.

---

## Service APIs

### Authentication API (auth-api)

**Base URL**: `http://localhost:8080` (development) | `https://auth.vpn.example.com` (production)

**Purpose**: User authentication, device management, and JWT token lifecycle

#### Core Endpoints

##### User Management
- `POST /users/register` - Register new user account
- `GET /users/profile` - Get user profile (requires auth)
- `PUT /users/profile` - Update user profile (requires auth)
- `DELETE /users/account` - Delete user account (requires auth)

##### Authentication Flow
- `POST /auth/login` - Email/password authentication
- `POST /auth/refresh` - Refresh access token
- `POST /auth/logout` - Invalidate refresh token
- `GET /.well-known/jwks.json` - JWT verification keys (public)

##### Device Management
- `POST /devices/register` - Register device public key
- `GET /devices` - List user devices (requires auth)
- `DELETE /devices/{device_id}` - Remove device (requires auth)

##### Device Flow (PKCE)
- `POST /auth/device/start` - Initiate device authorization
- `POST /auth/device/authorize` - User authorizes device
- `POST /auth/device/poll` - Device polls for authorization

##### GDPR Compliance
- `GET /gdpr/export` - Export user data (requires auth)
- `DELETE /gdpr/delete` - Delete user data (requires auth)

#### Authentication Schemes

**Bearer Token (JWT)**:
```http
Authorization: Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...
```

**Token Structure**:
```json
{
  "sub": "user_uuid",
  "email": "user@example.com",
  "device_id": "device_uuid",
  "iat": 1640995200,
  "exp": 1640996100,
  "aud": "vpn-client",
  "iss": "auth-api"
}
```

#### Example Requests

**User Registration**:
```bash
curl -X POST http://localhost:8080/users/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "SecurePassword123!",
    "consent": true,
    "tos_version": "v1.0",
    "residency": "EU"
  }'
```

**Login**:
```bash
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice@example.com",
    "password": "SecurePassword123!"
  }'
```

**Device Registration**:
```bash
curl -X POST http://localhost:8080/devices/register \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "device_public_key_b64": "base64_encoded_wireguard_public_key",
    "platform": "linux",
    "device_name": "Alice Laptop"
  }'
```

---

### Directory API (directory-api)

**Base URL**: `http://localhost:8081` (development) | `https://directory.vpn.example.com` (production)

**Purpose**: Node registry, region management, and client session allocation

#### Core Endpoints

##### Region Management
- `GET /regions` - List available regions
- `POST /regions` - Create new region (admin)
- `PUT /regions/{region_id}` - Update region (admin)
- `DELETE /regions/{region_id}` - Delete region (admin)

##### Node Registry
- `POST /nodes/register` - Node agent registration
- `POST /nodes/heartbeat` - Node health reporting
- `GET /nodes` - List nodes (admin)
- `PUT /nodes/{node_id}` - Update node configuration (admin)

##### Client Sessions
- `POST /mesh/client-config` - Allocate client session
- `DELETE /mesh/sessions/{session_id}` - Terminate session
- `GET /mesh/sessions` - List active sessions (admin)

##### Device Management
- `POST /devices/register` - Register client device (requires auth)

#### gRPC Interface

**Service**: `DirectoryService`
**Port**: `50051` (development) | `443` (production with mTLS)

**Methods**:
- `Register(NodeRegistration) → RegisterResponse`
- `Heartbeat(HeartbeatRequest) → HeartbeatResponse`
- `StreamConfig(ConfigRequest) → stream ConfigUpdate`

#### Example Requests

**List Regions**:
```bash
curl http://localhost:8081/regions
```

**Allocate Client Session**:
```bash
curl -X POST http://localhost:8081/mesh/client-config \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "region_id": "eu-berlin-1",
    "mode": "single-hop",
    "client_public_key": "base64_encoded_client_public_key"
  }'
```

**Node Registration**:
```bash
curl -X POST http://localhost:8081/nodes/register \
  -H "Content-Type: application/json" \
  -d '{
    "region_id": "eu-berlin-1",
    "public_key": "base64_encoded_node_public_key",
    "endpoint": "de-berlin.vpn.example.com:51820",
    "egress_ips": ["203.0.113.10", "203.0.113.11"]
  }'
```

---

### Admin API (admin-api)

**Base URL**: `http://localhost:8082` (development) | `https://admin.vpn.example.com` (production)

**Purpose**: Fleet management, user administration, and operational controls

#### Core Endpoints

##### User Administration
- `GET /admin/users` - List users with pagination
- `GET /admin/users/{user_id}` - Get user details
- `PUT /admin/users/{user_id}` - Update user (suspend, etc.)
- `DELETE /admin/users/{user_id}` - Delete user account

##### Fleet Management
- `GET /admin/nodes` - Node fleet overview
- `POST /admin/nodes/{node_id}/restart` - Restart node
- `PUT /admin/nodes/{node_id}/config` - Update node configuration

##### System Health
- `GET /admin/health` - System health dashboard
- `GET /admin/metrics` - Aggregate system metrics
- `GET /admin/audit` - Audit log access

#### Authentication

Admin API requires elevated privileges:
```http
Authorization: Bearer $ADMIN_TOKEN
X-Admin-Role: fleet-manager
```

---

## Shared Schemas

### Common Data Models

All APIs use consistent data models defined in `services/common/`:

#### User Model
```json
{
  "id": "uuid",
  "email": "string",
  "created_at": "datetime",
  "consent": "boolean",
  "tos_version": "string",
  "residency": "string",
  "is_active": "boolean"
}
```

#### Device Model
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "device_public_key": "string",
  "platform": "windows|macos|linux",
  "device_name": "string",
  "created_at": "datetime",
  "last_seen": "datetime"
}
```

#### Region Model
```json
{
  "id": "uuid",
  "country_code": "string",
  "city": "string",
  "status": "active|maintenance|disabled",
  "latency_score": "integer",
  "node_count": "integer"
}
```

#### Node Model
```json
{
  "id": "uuid",
  "region_id": "uuid",
  "public_key": "string",
  "endpoint": "string",
  "egress_ips": ["string"],
  "status": "online|offline|maintenance",
  "last_seen": "datetime",
  "active_connections": "integer"
}
```

#### Session Model
```json
{
  "id": "uuid",
  "user_id": "uuid",
  "device_id": "uuid",
  "node_id": "uuid",
  "client_ip": "string",
  "created_at": "datetime",
  "expires_at": "datetime"
}
```

---

## Protocol Buffers (gRPC)

### Directory Service

**File**: `services/common/protos/directory.proto`

```protobuf
service DirectoryService {
  rpc Register(NodeRegistration) returns (RegisterResponse);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
  rpc StreamConfig(ConfigRequest) returns (stream ConfigUpdate);
}

message NodeRegistration {
  string region_id = 1;
  string public_key = 2;
  string endpoint = 3;
  repeated string egress_ips = 4;
}

message ConfigUpdate {
  string session_id = 1;
  string client_public_key = 2;
  string client_ip = 3;
  repeated string allowed_ips = 4;
  bool remove = 5;
}
```

### Node Agent Communication

**Authentication**: mTLS with client certificates
**Compression**: gzip enabled
**Keepalive**: 30-second intervals

---

## OpenAPI Specifications

### Auto-Generated Documentation

Each service exposes OpenAPI specs at:
- auth-api: `http://localhost:8080/docs`
- directory-api: `http://localhost:8081/docs`
- admin-api: `http://localhost:8082/docs`

### Schema Validation

All APIs use Pydantic models for:
- Request/response validation
- Automatic OpenAPI generation
- Type safety and documentation

---

## Error Handling

### Standard Error Format

All APIs return errors in consistent format:

```json
{
  "error": {
    "code": "INVALID_CREDENTIALS",
    "message": "Invalid email or password",
    "details": {
      "field": "password",
      "constraint": "min_length"
    }
  }
}
```

### HTTP Status Codes

- `200` - Success
- `201` - Created
- `400` - Bad Request (validation error)
- `401` - Unauthorized (invalid/missing token)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `409` - Conflict (duplicate resource)
- `422` - Unprocessable Entity (business logic error)
- `429` - Too Many Requests (rate limited)
- `500` - Internal Server Error

### Error Codes

**Authentication Errors**:
- `INVALID_CREDENTIALS` - Wrong email/password
- `TOKEN_EXPIRED` - JWT token expired
- `TOKEN_INVALID` - Malformed or invalid JWT
- `DEVICE_NOT_FOUND` - Device not registered

**Validation Errors**:
- `VALIDATION_ERROR` - Request validation failed
- `MISSING_FIELD` - Required field missing
- `INVALID_FORMAT` - Field format invalid

**Business Logic Errors**:
- `USER_EXISTS` - Email already registered
- `REGION_FULL` - No available nodes in region
- `SESSION_LIMIT` - Maximum sessions exceeded
- `CONSENT_REQUIRED` - GDPR consent not provided

---

## Rate Limiting

### Default Limits

**Authentication Endpoints**:
- Login: 5 requests per minute per IP
- Registration: 3 requests per hour per IP
- Password reset: 3 requests per hour per email

**API Endpoints**:
- Authenticated: 100 requests per minute per user
- Public: 20 requests per minute per IP

### Headers

Rate limit information included in responses:
```http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1640995260
```

---

## Versioning Strategy

### API Versioning

- **Current Version**: v1
- **Header**: `Accept: application/vnd.vpn.v1+json`
- **URL Path**: `/v1/` prefix for explicit versioning
- **Backward Compatibility**: Maintained for 1 major version

### Schema Evolution

- **Additive Changes**: New optional fields (backward compatible)
- **Breaking Changes**: New major version required
- **Deprecation**: 6-month notice period for breaking changes

---

## Client Integration

### SDK Libraries

**Official SDKs**:
- Rust: `vpn-core` crate (in-repo)
- Python: `vpn-client-py` (planned)
- JavaScript: `@vpn/client-js` (planned)

### Authentication Flow

1. **Registration**: User creates account via `/users/register`
2. **Device Setup**: Client generates WireGuard keypair, registers via `/devices/register`
3. **Login**: Client authenticates via `/auth/login`, receives JWT tokens
4. **Session**: Client requests VPN config via `/mesh/client-config`
5. **Connection**: Client establishes WireGuard tunnel to assigned node
6. **Refresh**: Client refreshes tokens before expiry via `/auth/refresh`

### Configuration Management

**Client Configuration**:
```json
{
  "auth_endpoint": "https://auth.vpn.example.com",
  "directory_endpoint": "https://directory.vpn.example.com",
  "update_endpoint": "https://updates.vpn.example.com",
  "ca_certificates": ["base64_encoded_ca_cert"],
  "client_id": "desktop-client",
  "scopes": ["vpn:connect", "profile:read"]
}
```

---

## Testing and Development

### Mock Services

Development environment includes mock services for testing:
- `tools/mock-auth` - Simplified auth for integration tests
- `tools/mock-directory` - Local directory service
- `tools/chaos` - Chaos engineering for resilience testing

### API Testing

**Test Suites**:
- Unit tests: Service-level logic testing
- Integration tests: Cross-service workflow testing
- Contract tests: API schema validation
- Load tests: Performance and scalability testing

**Test Data**:
- Seeded test users and devices
- Mock regions and nodes
- Synthetic traffic patterns