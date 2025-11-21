# Development Guide

This comprehensive guide covers development workflows, coding standards, testing practices, and contribution guidelines for the VPN project.

---

## Development Overview

The VPN project follows modern development practices with:

- **Monorepo Structure**: All components in a single repository for tight integration
- **Multi-Language Stack**: Rust for performance-critical components, Python for APIs, TypeScript for UI
- **Container-First**: Docker and Docker Compose for consistent development environments
- **Test-Driven Development**: Comprehensive testing at unit, integration, and end-to-end levels
- **Continuous Integration**: Automated testing, linting, and security scanning
- **Documentation-Driven**: All features documented before implementation

---

## Getting Started

### Prerequisites

Before starting development, ensure you have:

- **Git**: Version control system
- **Docker & Docker Compose**: Container runtime and orchestration
- **Rust**: 1.70+ with `rustfmt` and `clippy` components
- **Python**: 3.11+ with `pip` and `poetry`
- **Node.js**: 18+ with `npm`
- **Make**: Build automation tool

### Initial Setup

#### 1. Clone Repository

```bash
git clone https://github.com/example/vpn-mvp.git
cd vpn-mvp
```

#### 2. Run Bootstrap Script

```bash
# Install all dependencies and setup development environment
make bootstrap

# This script will:
# - Install Rust toolchain and components
# - Setup Python virtual environment
# - Install Node.js dependencies
# - Generate development certificates
# - Setup pre-commit hooks
# - Create environment files
```

#### 3. Start Development Stack

```bash
# Start all services in development mode
docker compose up --build

# Or start specific services
docker compose up postgres redis auth-api
```

#### 4. Verify Setup

```bash
# Check service health
make health-check

# Run tests to ensure everything works
make test
```

### Development Environment Structure

```
vpn-mvp/
├── client/                    # Desktop client components
│   ├── vpn-core/             # Rust networking core
│   ├── vpn-gui/              # Tauri desktop application
│   ├── platform-linux/       # Linux-specific implementations
│   ├── platform-macos/       # macOS-specific implementations
│   └── platform-windows/     # Windows-specific implementations
├── services/                  # Backend API services
│   ├── auth-api/             # Authentication service
│   ├── directory-api/        # Node directory service
│   ├── admin-api/            # Administrative interface
│   └── common/               # Shared schemas and utilities
├── nodes/                     # Node agent implementation
│   └── agent/                # Rust-based WireGuard agent
├── infra/                     # Infrastructure as code
│   ├── docker/               # Container definitions
│   ├── compose/              # Docker Compose configurations
│   └── migrations/           # Database migrations
├── tools/                     # Development and testing tools
│   ├── chaos/                # Chaos engineering tools
│   ├── pki/                  # Certificate management
│   └── testing/              # Test utilities
├── docs/                      # Documentation
└── scripts/                   # Automation scripts
```

---

## Development Workflows

### Feature Development Workflow

#### 1. Create Feature Branch

```bash
# Create and switch to feature branch
git checkout -b feature/user-authentication

# Or for bug fixes
git checkout -b fix/token-expiration-bug
```

#### 2. Implement Feature

Follow the TDD approach:

1. **Write Tests First**: Create failing tests that define expected behavior
2. **Implement Code**: Write minimal code to make tests pass
3. **Refactor**: Improve code quality while keeping tests green
4. **Document**: Update documentation and add code comments

#### 3. Test Changes

```bash
# Run all tests
make test

# Run specific test suites
cargo test --package vpn-core
pytest services/auth-api/tests/
npm test --prefix client/vpn-gui/ui

# Run integration tests
make test-integration

# Run linting and formatting
make lint
make fmt
```

#### 4. Commit Changes

```bash
# Stage changes
git add .

# Commit with descriptive message
git commit -m "feat(auth): implement JWT token refresh mechanism

- Add refresh token rotation for enhanced security
- Implement automatic token refresh in client
- Add tests for token lifecycle management
- Update API documentation

Closes #123"
```

#### 5. Create Pull Request

```bash
# Push feature branch
git push origin feature/user-authentication

# Create pull request via GitHub CLI or web interface
gh pr create --title "Implement JWT token refresh mechanism" \
             --body "Detailed description of changes..."
```

### Code Review Process

#### Pull Request Requirements

Before merging, ensure:

- [ ] All tests pass (unit, integration, e2e)
- [ ] Code coverage meets minimum threshold (80%)
- [ ] Linting and formatting checks pass
- [ ] Security scan shows no new vulnerabilities
- [ ] Documentation is updated
- [ ] At least one approving review from maintainer

#### Review Checklist

**Functionality**:
- [ ] Code solves the intended problem
- [ ] Edge cases are handled appropriately
- [ ] Error handling is comprehensive
- [ ] Performance implications are considered

**Code Quality**:
- [ ] Code is readable and well-structured
- [ ] Functions and variables have descriptive names
- [ ] Complex logic is commented
- [ ] No code duplication

**Security**:
- [ ] Input validation is present
- [ ] Sensitive data is handled securely
- [ ] Authentication/authorization is correct
- [ ] No secrets in code or logs

**Testing**:
- [ ] New functionality has corresponding tests
- [ ] Tests cover happy path and error cases
- [ ] Integration points are tested
- [ ] Performance tests for critical paths

---

## Coding Standards

### Rust Code Standards

#### Project Structure

```rust
// Standard crate structure
src/
├── lib.rs              // Public API and re-exports
├── main.rs             // Binary entry point (if applicable)
├── error.rs            // Error types and handling
├── config.rs           // Configuration structures
├── types.rs            // Common type definitions
└── modules/            // Feature-specific modules
    ├── mod.rs          // Module declarations
    ├── auth.rs         // Authentication logic
    └── network.rs      // Network operations
```

#### Code Style

```rust
// Use descriptive names
pub struct ConnectionManager {
    active_connections: HashMap<UserId, Connection>,
    max_connections: usize,
}

// Implement proper error handling
pub async fn establish_connection(&mut self, user_id: UserId) -> Result<Connection, ConnectionError> {
    if self.active_connections.len() >= self.max_connections {
        return Err(ConnectionError::MaxConnectionsReached);
    }
    
    let connection = Connection::new(user_id).await
        .map_err(ConnectionError::EstablishmentFailed)?;
    
    self.active_connections.insert(user_id, connection.clone());
    
    Ok(connection)
}

// Use proper documentation
/// Manages VPN connections for authenticated users.
/// 
/// The ConnectionManager handles the lifecycle of VPN connections,
/// including establishment, monitoring, and cleanup.
/// 
/// # Examples
/// 
/// ```rust
/// let mut manager = ConnectionManager::new(100);
/// let connection = manager.establish_connection(user_id).await?;
/// ```
impl ConnectionManager {
    /// Creates a new connection manager with the specified capacity.
    pub fn new(max_connections: usize) -> Self {
        Self {
            active_connections: HashMap::new(),
            max_connections,
        }
    }
}
```

#### Testing Standards

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_connection_establishment_success() {
        let mut manager = ConnectionManager::new(10);
        let user_id = UserId::new();
        
        let result = manager.establish_connection(user_id).await;
        
        assert!(result.is_ok());
        assert_eq!(manager.active_connections.len(), 1);
    }
    
    #[tokio::test]
    async fn test_connection_limit_exceeded() {
        let mut manager = ConnectionManager::new(1);
        let user1 = UserId::new();
        let user2 = UserId::new();
        
        // First connection should succeed
        manager.establish_connection(user1).await.unwrap();
        
        // Second connection should fail
        let result = manager.establish_connection(user2).await;
        assert!(matches!(result, Err(ConnectionError::MaxConnectionsReached)));
    }
}
```

### Python Code Standards

#### Project Structure

```python
# Standard FastAPI service structure
app/
├── __init__.py
├── main.py              # FastAPI application entry point
├── config.py            # Configuration and settings
├── dependencies.py      # Dependency injection
├── models/              # Database models
│   ├── __init__.py
│   ├── user.py
│   └── device.py
├── schemas/             # Pydantic schemas
│   ├── __init__.py
│   ├── user.py
│   └── auth.py
├── routers/             # API route handlers
│   ├── __init__.py
│   ├── auth.py
│   └── users.py
├── services/            # Business logic
│   ├── __init__.py
│   ├── auth_service.py
│   └── user_service.py
└── utils/               # Utility functions
    ├── __init__.py
    ├── security.py
    └── database.py
```

#### Code Style

```python
from typing import Optional, List
from pydantic import BaseModel, EmailStr, validator
from fastapi import HTTPException, status

class UserCreate(BaseModel):
    """Schema for user creation requests."""
    
    email: EmailStr
    password: str
    consent: bool
    tos_version: str
    
    @validator('password')
    def validate_password(cls, v: str) -> str:
        """Validate password meets security requirements."""
        if len(v) < 12:
            raise ValueError('Password must be at least 12 characters long')
        
        if not any(c.isupper() for c in v):
            raise ValueError('Password must contain at least one uppercase letter')
        
        if not any(c.islower() for c in v):
            raise ValueError('Password must contain at least one lowercase letter')
        
        if not any(c.isdigit() for c in v):
            raise ValueError('Password must contain at least one digit')
        
        return v

class AuthService:
    """Service for handling user authentication operations."""
    
    def __init__(self, db: Database, redis: Redis):
        self.db = db
        self.redis = redis
    
    async def register_user(self, user_data: UserCreate) -> User:
        """Register a new user account.
        
        Args:
            user_data: User registration information
            
        Returns:
            Created user object
            
        Raises:
            HTTPException: If email already exists or validation fails
        """
        # Check if user already exists
        existing_user = await self.db.get_user_by_email(user_data.email)
        if existing_user:
            raise HTTPException(
                status_code=status.HTTP_409_CONFLICT,
                detail="Email already registered"
            )
        
        # Hash password
        password_hash = hash_password(user_data.password)
        
        # Create user
        user = await self.db.create_user(
            email=user_data.email,
            password_hash=password_hash,
            consent=user_data.consent,
            tos_version=user_data.tos_version
        )
        
        return user
```

#### Testing Standards

```python
import pytest
from fastapi.testclient import TestClient
from unittest.mock import AsyncMock, patch

from app.main import app
from app.services.auth_service import AuthService
from app.schemas.user import UserCreate

client = TestClient(app)

class TestAuthService:
    """Test suite for AuthService."""
    
    @pytest.fixture
    def auth_service(self):
        """Create AuthService instance with mocked dependencies."""
        db_mock = AsyncMock()
        redis_mock = AsyncMock()
        return AuthService(db_mock, redis_mock)
    
    @pytest.fixture
    def valid_user_data(self):
        """Valid user registration data."""
        return UserCreate(
            email="test@example.com",
            password="SecurePassword123!",
            consent=True,
            tos_version="v1.0"
        )
    
    async def test_register_user_success(self, auth_service, valid_user_data):
        """Test successful user registration."""
        # Setup mocks
        auth_service.db.get_user_by_email.return_value = None
        auth_service.db.create_user.return_value = User(
            id="user-123",
            email=valid_user_data.email,
            created_at=datetime.utcnow()
        )
        
        # Execute
        result = await auth_service.register_user(valid_user_data)
        
        # Verify
        assert result.email == valid_user_data.email
        auth_service.db.get_user_by_email.assert_called_once_with(valid_user_data.email)
        auth_service.db.create_user.assert_called_once()
    
    async def test_register_user_duplicate_email(self, auth_service, valid_user_data):
        """Test registration with existing email."""
        # Setup mocks
        auth_service.db.get_user_by_email.return_value = User(
            id="existing-user",
            email=valid_user_data.email
        )
        
        # Execute and verify
        with pytest.raises(HTTPException) as exc_info:
            await auth_service.register_user(valid_user_data)
        
        assert exc_info.value.status_code == 409
        assert "Email already registered" in exc_info.value.detail

class TestAuthAPI:
    """Integration tests for auth API endpoints."""
    
    def test_register_user_endpoint(self):
        """Test user registration endpoint."""
        response = client.post("/users/register", json={
            "email": "test@example.com",
            "password": "SecurePassword123!",
            "consent": True,
            "tos_version": "v1.0"
        })
        
        assert response.status_code == 201
        data = response.json()
        assert "user_id" in data
        assert data["email"] == "test@example.com"
    
    def test_register_user_invalid_password(self):
        """Test registration with invalid password."""
        response = client.post("/users/register", json={
            "email": "test@example.com",
            "password": "weak",
            "consent": True,
            "tos_version": "v1.0"
        })
        
        assert response.status_code == 422
        assert "Password must be at least 12 characters" in response.text
```

### TypeScript Code Standards

#### Project Structure

```typescript
// React/Tauri frontend structure
src/
├── components/          // Reusable UI components
│   ├── common/         // Generic components
│   ├── forms/          // Form components
│   └── layout/         // Layout components
├── pages/              // Page components
├── hooks/              // Custom React hooks
├── services/           // API and external services
├── stores/             // State management
├── types/              // TypeScript type definitions
├── utils/              // Utility functions
└── styles/             // CSS and styling
```

#### Code Style

```typescript
// Type definitions
interface User {
  id: string;
  email: string;
  createdAt: Date;
  isActive: boolean;
}

interface ConnectionStatus {
  state: 'disconnected' | 'connecting' | 'connected' | 'disconnecting';
  server?: ServerInfo;
  connectedSince?: Date;
  bytesTransferred: {
    sent: number;
    received: number;
  };
}

// React component with proper typing
interface ConnectionPanelProps {
  onConnect: (serverId: string) => Promise<void>;
  onDisconnect: () => Promise<void>;
  status: ConnectionStatus;
  servers: ServerInfo[];
}

export const ConnectionPanel: React.FC<ConnectionPanelProps> = ({
  onConnect,
  onDisconnect,
  status,
  servers
}) => {
  const [selectedServer, setSelectedServer] = useState<string>('');
  const [isLoading, setIsLoading] = useState(false);

  const handleConnect = useCallback(async () => {
    if (!selectedServer || isLoading) return;

    setIsLoading(true);
    try {
      await onConnect(selectedServer);
    } catch (error) {
      console.error('Connection failed:', error);
      // Show error notification
    } finally {
      setIsLoading(false);
    }
  }, [selectedServer, isLoading, onConnect]);

  return (
    <div className="connection-panel">
      <StatusIndicator status={status.state} />
      
      {status.state === 'disconnected' && (
        <div className="connection-controls">
          <ServerSelector
            servers={servers}
            value={selectedServer}
            onChange={setSelectedServer}
          />
          <Button
            onClick={handleConnect}
            disabled={!selectedServer || isLoading}
            loading={isLoading}
          >
            Connect
          </Button>
        </div>
      )}
      
      {status.state === 'connected' && (
        <div className="connection-info">
          <ConnectionStats stats={status.bytesTransferred} />
          <Button onClick={onDisconnect} variant="danger">
            Disconnect
          </Button>
        </div>
      )}
    </div>
  );
};
```

#### Testing Standards

```typescript
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import { ConnectionPanel } from './ConnectionPanel';

describe('ConnectionPanel', () => {
  const mockServers = [
    { id: 'server-1', name: 'US East', location: 'New York' },
    { id: 'server-2', name: 'EU West', location: 'London' }
  ];

  const defaultProps = {
    onConnect: vi.fn(),
    onDisconnect: vi.fn(),
    status: { state: 'disconnected' as const, bytesTransferred: { sent: 0, received: 0 } },
    servers: mockServers
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders connection controls when disconnected', () => {
    render(<ConnectionPanel {...defaultProps} />);
    
    expect(screen.getByText('Connect')).toBeInTheDocument();
    expect(screen.getByRole('combobox')).toBeInTheDocument();
  });

  it('calls onConnect when connect button is clicked', async () => {
    render(<ConnectionPanel {...defaultProps} />);
    
    // Select a server
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'server-1' } });
    
    // Click connect
    fireEvent.click(screen.getByText('Connect'));
    
    await waitFor(() => {
      expect(defaultProps.onConnect).toHaveBeenCalledWith('server-1');
    });
  });

  it('shows connection stats when connected', () => {
    const connectedStatus = {
      state: 'connected' as const,
      server: mockServers[0],
      connectedSince: new Date(),
      bytesTransferred: { sent: 1024, received: 2048 }
    };

    render(<ConnectionPanel {...defaultProps} status={connectedStatus} />);
    
    expect(screen.getByText('Disconnect')).toBeInTheDocument();
    expect(screen.getByText(/1\.0 KB/)).toBeInTheDocument(); // Sent bytes
    expect(screen.getByText(/2\.0 KB/)).toBeInTheDocument(); // Received bytes
  });

  it('disables connect button when no server selected', () => {
    render(<ConnectionPanel {...defaultProps} />);
    
    const connectButton = screen.getByText('Connect');
    expect(connectButton).toBeDisabled();
  });
});
```

---

## Testing Practices

### Test Strategy

The project follows a comprehensive testing pyramid:

1. **Unit Tests (70%)**: Fast, isolated tests for individual functions/methods
2. **Integration Tests (20%)**: Tests for component interactions
3. **End-to-End Tests (10%)**: Full system workflow tests

### Unit Testing

#### Rust Unit Tests

```rust
// In-module tests
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[test]
    fn test_password_hashing() {
        let password = "test_password";
        let hash = hash_password(password);
        
        assert_ne!(hash, password);
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong_password", &hash));
    }
    
    #[tokio::test]
    async fn test_async_operation() {
        let result = async_function().await;
        assert!(result.is_ok());
    }
}

// Separate test files for complex modules
// tests/integration_test.rs
use vpn_core::*;

#[tokio::test]
async fn test_connection_manager_integration() {
    let manager = ConnectionManager::new(10);
    // Test complex scenarios
}
```

#### Python Unit Tests

```python
# pytest configuration in pyproject.toml
[tool.pytest.ini_options]
testpaths = ["tests"]
python_files = ["test_*.py", "*_test.py"]
python_classes = ["Test*"]
python_functions = ["test_*"]
addopts = [
    "--strict-markers",
    "--strict-config",
    "--cov=app",
    "--cov-report=term-missing",
    "--cov-report=html",
    "--cov-fail-under=80"
]

# Test fixtures
@pytest.fixture
async def db_session():
    """Create test database session."""
    async with AsyncSession(test_engine) as session:
        yield session
        await session.rollback()

@pytest.fixture
def auth_service(db_session):
    """Create AuthService with test dependencies."""
    return AuthService(db_session, mock_redis)

# Parameterized tests
@pytest.mark.parametrize("password,expected", [
    ("short", False),
    ("nouppercase123!", False),
    ("NOLOWERCASE123!", False),
    ("NoNumbers!", False),
    ("ValidPassword123!", True),
])
def test_password_validation(password, expected):
    result = is_valid_password(password)
    assert result == expected
```

### Integration Testing

#### API Integration Tests

```python
# tests/integration/test_auth_flow.py
import pytest
from httpx import AsyncClient

@pytest.mark.asyncio
class TestAuthenticationFlow:
    """Test complete authentication workflows."""
    
    async def test_user_registration_and_login(self, client: AsyncClient):
        """Test complete user registration and login flow."""
        
        # Register user
        registration_data = {
            "email": "integration@example.com",
            "password": "IntegrationTest123!",
            "consent": True,
            "tos_version": "v1.0"
        }
        
        response = await client.post("/users/register", json=registration_data)
        assert response.status_code == 201
        user_data = response.json()
        assert "user_id" in user_data
        
        # Login with registered user
        login_data = {
            "email": registration_data["email"],
            "password": registration_data["password"]
        }
        
        response = await client.post("/auth/login", json=login_data)
        assert response.status_code == 200
        tokens = response.json()
        assert "access_token" in tokens
        assert "refresh_token" in tokens
        
        # Use access token to access protected endpoint
        headers = {"Authorization": f"Bearer {tokens['access_token']}"}
        response = await client.get("/users/profile", headers=headers)
        assert response.status_code == 200
        profile = response.json()
        assert profile["email"] == registration_data["email"]
```

#### Database Integration Tests

```python
# tests/integration/test_database.py
import pytest
from sqlalchemy.ext.asyncio import AsyncSession

@pytest.mark.asyncio
class TestDatabaseOperations:
    """Test database operations and constraints."""
    
    async def test_user_creation_and_retrieval(self, db_session: AsyncSession):
        """Test user CRUD operations."""
        
        # Create user
        user_data = {
            "email": "db_test@example.com",
            "password_hash": "hashed_password",
            "consent": True,
            "tos_version": "v1.0"
        }
        
        user = User(**user_data)
        db_session.add(user)
        await db_session.commit()
        await db_session.refresh(user)
        
        # Retrieve user
        retrieved_user = await db_session.get(User, user.id)
        assert retrieved_user is not None
        assert retrieved_user.email == user_data["email"]
        
        # Test unique constraint
        duplicate_user = User(**user_data)
        db_session.add(duplicate_user)
        
        with pytest.raises(IntegrityError):
            await db_session.commit()
```

### End-to-End Testing

#### Playwright E2E Tests

```typescript
// tests/e2e/connection-flow.spec.ts
import { test, expect } from '@playwright/test';

test.describe('VPN Connection Flow', () => {
  test.beforeEach(async ({ page }) => {
    // Setup test environment
    await page.goto('/');
    await page.waitForLoadState('networkidle');
  });

  test('complete connection workflow', async ({ page }) => {
    // Login
    await page.click('[data-testid="login-button"]');
    await page.fill('[data-testid="email-input"]', 'test@example.com');
    await page.fill('[data-testid="password-input"]', 'TestPassword123!');
    await page.click('[data-testid="submit-login"]');
    
    // Wait for dashboard
    await expect(page.locator('[data-testid="dashboard"]')).toBeVisible();
    
    // Select server and connect
    await page.selectOption('[data-testid="server-selector"]', 'us-east-1');
    await page.click('[data-testid="connect-button"]');
    
    // Wait for connection
    await expect(page.locator('[data-testid="status-connected"]')).toBeVisible({ timeout: 30000 });
    
    // Verify connection stats
    await expect(page.locator('[data-testid="connection-stats"]')).toBeVisible();
    
    // Disconnect
    await page.click('[data-testid="disconnect-button"]');
    await expect(page.locator('[data-testid="status-disconnected"]')).toBeVisible();
  });

  test('handles connection errors gracefully', async ({ page }) => {
    // Mock network failure
    await page.route('**/api/mesh/client-config', route => {
      route.fulfill({ status: 500, body: 'Internal Server Error' });
    });
    
    // Attempt connection
    await page.selectOption('[data-testid="server-selector"]', 'us-east-1');
    await page.click('[data-testid="connect-button"]');
    
    // Verify error handling
    await expect(page.locator('[data-testid="error-notification"]')).toBeVisible();
    await expect(page.locator('[data-testid="status-disconnected"]')).toBeVisible();
  });
});
```

### Performance Testing

#### Load Testing with Locust

```python
# tests/performance/locustfile.py
from locust import HttpUser, task, between
import random
import string

class VPNAPIUser(HttpUser):
    wait_time = between(1, 3)
    
    def on_start(self):
        """Setup user session."""
        self.register_and_login()
    
    def register_and_login(self):
        """Register and login user."""
        # Generate unique email
        email = f"perf_test_{''.join(random.choices(string.ascii_lowercase, k=8))}@example.com"
        
        # Register
        response = self.client.post("/users/register", json={
            "email": email,
            "password": "PerfTest123!",
            "consent": True,
            "tos_version": "v1.0"
        })
        
        if response.status_code == 201:
            # Login
            response = self.client.post("/auth/login", json={
                "email": email,
                "password": "PerfTest123!"
            })
            
            if response.status_code == 200:
                tokens = response.json()
                self.access_token = tokens["access_token"]
                self.headers = {"Authorization": f"Bearer {self.access_token}"}
    
    @task(3)
    def get_regions(self):
        """Test region listing performance."""
        self.client.get("/regions", headers=self.headers)
    
    @task(2)
    def get_profile(self):
        """Test profile retrieval performance."""
        self.client.get("/users/profile", headers=self.headers)
    
    @task(1)
    def allocate_session(self):
        """Test session allocation performance."""
        # Get available regions first
        response = self.client.get("/regions", headers=self.headers)
        if response.status_code == 200:
            regions = response.json()
            if regions:
                region_id = random.choice(regions)["id"]
                
                # Allocate session
                self.client.post("/mesh/client-config", json={
                    "region_id": region_id,
                    "mode": "single-hop",
                    "client_public_key": "test_public_key"
                }, headers=self.headers)
```

---

## Debugging and Troubleshooting

### Development Debugging

#### Service Debugging

```bash
# Debug specific service with attached debugger
make debug-service SERVICE=auth-api PORT=5678

# View service logs
docker logs -f vpn-auth-api-1

# Execute commands in service container
docker exec -it vpn-auth-api-1 bash

# Check service health
curl http://localhost:8080/healthz
```

#### Database Debugging

```bash
# Connect to development database
docker exec -it vpn-postgres-1 psql -U postgres -d vpn_db

# View active connections
SELECT * FROM pg_stat_activity;

# Check table sizes
SELECT schemaname,tablename,pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size 
FROM pg_tables 
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

# View recent migrations
SELECT * FROM alembic_version;
```

#### Network Debugging

```bash
# Check WireGuard status
docker exec vpn-node-agent-1 wg show

# Test connectivity between services
docker exec vpn-auth-api-1 curl -f http://directory-api:8081/healthz

# Monitor network traffic
docker exec vpn-node-agent-1 tcpdump -i wg0

# Check DNS resolution
docker exec vpn-node-agent-1 nslookup directory-api
```

### Common Issues and Solutions

#### Service Won't Start

```bash
# Check Docker daemon
sudo systemctl status docker

# Check port conflicts
netstat -tulpn | grep :8080

# Check environment variables
docker exec vpn-auth-api-1 env | grep DATABASE_URL

# Rebuild with no cache
docker compose build --no-cache auth-api
```

#### Database Connection Issues

```bash
# Check database container status
docker ps | grep postgres

# Test database connectivity
docker exec vpn-auth-api-1 python -c "
import asyncpg
import asyncio
async def test():
    conn = await asyncpg.connect('$DATABASE_URL')
    print('Connected successfully')
    await conn.close()
asyncio.run(test())
"

# Reset database
docker compose down postgres
docker volume rm vpn-mvp_postgres_data
docker compose up -d postgres
```

#### Authentication Issues

```bash
# Check JWT configuration
docker exec vpn-auth-api-1 python -c "
from app.config import settings
print(f'JWT Algorithm: {settings.JWT_ALGORITHM}')
print(f'Token Expiry: {settings.JWT_ACCESS_TOKEN_EXPIRE_MINUTES}')
"

# Verify JWKS endpoint
curl http://localhost:8080/.well-known/jwks.json

# Test token generation
curl -X POST http://localhost:8080/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com","password":"TestPassword123!"}'
```

---

## Contributing Guidelines

### Contribution Process

#### 1. Issue Creation

Before starting work:

- Check existing issues to avoid duplication
- Create detailed issue with:
  - Clear problem description
  - Expected behavior
  - Steps to reproduce (for bugs)
  - Acceptance criteria (for features)

#### 2. Development

- Fork repository and create feature branch
- Follow coding standards and conventions
- Write tests for new functionality
- Update documentation as needed
- Ensure all checks pass locally

#### 3. Pull Request

- Create PR with descriptive title and detailed description
- Link related issues
- Request review from appropriate maintainers
- Address feedback promptly
- Ensure CI/CD pipeline passes

### Code Review Guidelines

#### For Authors

- Keep PRs focused and reasonably sized
- Write clear commit messages
- Test changes thoroughly
- Update documentation
- Respond to feedback constructively

#### For Reviewers

- Review promptly (within 24-48 hours)
- Focus on code quality, security, and maintainability
- Provide constructive feedback
- Approve when satisfied with changes
- Use GitHub's suggestion feature for minor fixes

### Release Process

#### Version Numbering

Follow Semantic Versioning (SemVer):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

#### Release Workflow

```bash
# Create release branch
git checkout -b release/v1.2.0

# Update version numbers
./scripts/update_version.sh v1.2.0

# Run full test suite
make test-all

# Create release PR
gh pr create --title "Release v1.2.0" --body "Release notes..."

# After approval, merge and tag
git checkout main
git merge release/v1.2.0
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin main --tags

# GitHub Actions will handle building and publishing
```

---

## Development Tools and Utilities

### Code Quality Tools

#### Pre-commit Hooks

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.4.0
    hooks:
      - id: trailing-whitespace
      - id: end-of-file-fixer
      - id: check-yaml
      - id: check-added-large-files

  - repo: https://github.com/psf/black
    rev: 23.1.0
    hooks:
      - id: black
        files: ^services/.*\.py$

  - repo: https://github.com/charliermarsh/ruff-pre-commit
    rev: v0.0.254
    hooks:
      - id: ruff
        files: ^services/.*\.py$

  - repo: local
    hooks:
      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt --all --
        language: system
        files: \.rs$
        pass_filenames: false

      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy --workspace -- -D warnings
        language: system
        files: \.rs$
        pass_filenames: false
```

#### IDE Configuration

**VS Code Settings** (`.vscode/settings.json`):

```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "python.defaultInterpreterPath": ".venv/bin/python",
  "python.linting.enabled": true,
  "python.linting.ruffEnabled": true,
  "python.formatting.provider": "black",
  "typescript.preferences.importModuleSpecifier": "relative",
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll": true,
    "source.organizeImports": true
  }
}
```

### Development Scripts

#### Database Reset Script

```bash
#!/bin/bash
# scripts/reset_dev_db.sh

set -euo pipefail

echo "🗄️ Resetting development database..."

# Stop services
docker compose stop auth-api directory-api admin-api

# Remove database volume
docker compose down postgres
docker volume rm vpn-mvp_postgres_data || true

# Start fresh database
docker compose up -d postgres

# Wait for database to be ready
echo "Waiting for database to be ready..."
sleep 10

# Run migrations
for service in auth-api directory-api admin-api; do
    echo "Running migrations for $service..."
    cd "services/$service"
    alembic upgrade head
    cd ../..
done

# Seed test data
python scripts/seed_test_data.py

# Restart services
docker compose up -d auth-api directory-api admin-api

echo "✅ Database reset complete"
```

#### Test Data Seeding

```python
#!/usr/bin/env python3
# scripts/seed_test_data.py

import asyncio
import asyncpg
import bcrypt
from datetime import datetime

async def seed_database():
    """Seed database with test data."""
    
    conn = await asyncpg.connect("postgresql://postgres:example@localhost:5432/postgres")
    
    try:
        # Create test users
        users = [
            {
                "email": "alice@example.com",
                "password": "TestPassword123!",
                "consent": True,
                "tos_version": "v1.0"
            },
            {
                "email": "bob@example.com", 
                "password": "TestPassword123!",
                "consent": True,
                "tos_version": "v1.0"
            }
        ]
        
        for user in users:
            # Hash password
            password_hash = bcrypt.hashpw(
                user["password"].encode('utf-8'), 
                bcrypt.gensalt()
            ).decode('utf-8')
            
            # Insert user
            await conn.execute("""
                INSERT INTO users (email, password_hash, consent, tos_version, created_at)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (email) DO NOTHING
            """, user["email"], password_hash, user["consent"], 
                user["tos_version"], datetime.utcnow())
        
        # Create test regions
        regions = [
            {"country_code": "US", "city": "New York", "status": "active"},
            {"country_code": "US", "city": "Los Angeles", "status": "active"},
            {"country_code": "DE", "city": "Frankfurt", "status": "active"},
            {"country_code": "GB", "city": "London", "status": "active"}
        ]
        
        for region in regions:
            await conn.execute("""
                INSERT INTO regions (country_code, city, status, created_at)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (country_code, city) DO NOTHING
            """, region["country_code"], region["city"], 
                region["status"], datetime.utcnow())
        
        print("✅ Test data seeded successfully")
        
    finally:
        await conn.close()

if __name__ == "__main__":
    asyncio.run(seed_database())
```