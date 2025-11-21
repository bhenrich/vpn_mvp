# Development and Testing Tools

This section documents the development tools, testing frameworks, and automation utilities used to build, test, and maintain the VPN system.

---

## Tools Overview

The VPN project includes comprehensive tooling for:
- **Development**: Local setup, code generation, and debugging
- **Testing**: Unit tests, integration tests, and chaos engineering
- **Security**: PKI management, vulnerability scanning, and compliance checks
- **Operations**: Deployment automation, monitoring, and maintenance
- **Quality Assurance**: Code formatting, linting, and static analysis

---

## Development Tools

### Local Development Environment

#### Bootstrap Script

The `make bootstrap` command sets up the complete development environment:

```bash
#!/bin/bash
# scripts/bootstrap.sh

set -euo pipefail

echo "🚀 Setting up VPN development environment..."

# Install system dependencies
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    sudo apt-get update
    sudo apt-get install -y build-essential pkg-config libssl-dev
elif [[ "$OSTYPE" == "darwin"* ]]; then
    brew install pkg-config openssl
fi

# Install Rust toolchain
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# Install required Rust components
rustup component add rustfmt clippy
rustup target add x86_64-pc-windows-gnu  # For cross-compilation

# Install Python dependencies
if ! command -v python3.11 &> /dev/null; then
    echo "Python 3.11+ required. Please install and retry."
    exit 1
fi

python3 -m pip install --upgrade pip
pip install pre-commit poetry

# Install Node.js dependencies (for Tauri UI)
if ! command -v node &> /dev/null; then
    curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
    sudo apt-get install -y nodejs
fi

# Install Docker and Docker Compose
if ! command -v docker &> /dev/null; then
    curl -fsSL https://get.docker.com | sh
    sudo usermod -aG docker $USER
fi

# Setup pre-commit hooks
pre-commit install

# Generate development certificates
./tools/pki/gen_dev_certs.py

# Create environment files
for service in auth-api directory-api admin-api; do
    if [ ! -f "services/$service/.env" ]; then
        cp "services/$service/env.example" "services/$service/.env"
    fi
done

echo "✅ Development environment setup complete!"
echo "Run 'docker compose up' to start the development stack."
```

#### Makefile Targets

```makefile
# Makefile - Development commands

.PHONY: help bootstrap build test lint fmt clean

help: ## Show this help message
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

bootstrap: ## Setup development environment
	./scripts/bootstrap.sh

build: ## Build all services and clients
	docker compose build
	cargo build --workspace
	cd clients/desktop/ui && npm run build

test: ## Run all tests
	cargo test --workspace
	python -m pytest services/
	cd clients/desktop/ui && npm test

lint: ## Run linters
	cargo clippy --workspace -- -D warnings
	ruff check services/
	cd clients/desktop/ui && npm run lint

fmt: ## Format code
	cargo fmt --all
	ruff format services/
	cd clients/desktop/ui && npm run format

clean: ## Clean build artifacts
	cargo clean
	docker compose down -v
	docker system prune -f

up: ## Start development stack
	docker compose up --build

down: ## Stop development stack
	docker compose down

logs: ## Show service logs
	docker compose logs -f

health: ## Check service health
	./scripts/health_check.sh

reset: ## Reset development environment
	docker compose down -v
	docker system prune -f
	./scripts/reset_dev_db.sh
```

### Code Generation

#### Protocol Buffer Generation

```bash
#!/bin/bash
# scripts/generate_protos.sh

set -euo pipefail

PROTO_DIR="services/common/protos"
OUTPUT_DIR="services/common/generated"

# Generate Python stubs
python -m grpc_tools.protoc \
    --proto_path=$PROTO_DIR \
    --python_out=$OUTPUT_DIR \
    --grpc_python_out=$OUTPUT_DIR \
    $PROTO_DIR/*.proto

# Generate Rust stubs
cd nodes/agent
cargo build  # Uses build.rs to generate Rust code

echo "✅ Protocol buffers generated successfully"
```

#### OpenAPI Client Generation

```bash
#!/bin/bash
# scripts/generate_clients.sh

# Generate TypeScript client for desktop UI
openapi-generator-cli generate \
    -i http://localhost:8080/openapi.json \
    -g typescript-axios \
    -o clients/desktop/ui/src/generated/auth-api

# Generate Python client for testing
openapi-generator-cli generate \
    -i http://localhost:8081/openapi.json \
    -g python \
    -o tools/testing/clients/directory-api
```

### Debugging Tools

#### Service Debugging

```bash
#!/bin/bash
# scripts/debug_service.sh

SERVICE=${1:-auth-api}
PORT=${2:-5678}

echo "🐛 Starting debug session for $SERVICE on port $PORT"

# Start service with debugger
docker compose run --rm \
    -p $PORT:$PORT \
    -e PYTHONPATH=/app \
    -e DEBUGPY_WAIT_FOR_CLIENT=1 \
    $SERVICE \
    python -m debugpy --listen 0.0.0.0:$PORT --wait-for-client -m uvicorn app.main:app --host 0.0.0.0 --port 8080 --reload

echo "Connect your IDE debugger to localhost:$PORT"
```

#### Network Debugging

```bash
#!/bin/bash
# scripts/debug_network.sh

echo "🔍 Network debugging tools"

# Check WireGuard interface
echo "=== WireGuard Status ==="
docker exec vpn-node-agent wg show

# Check iptables rules
echo "=== IPTables Rules ==="
docker exec vpn-node-agent iptables -L -n -v

# Test connectivity
echo "=== Connectivity Tests ==="
docker exec vpn-node-agent ping -c 3 8.8.8.8
docker exec vpn-node-agent nslookup google.com

# Check DNS resolution
echo "=== DNS Resolution ==="
docker exec vpn-node-agent cat /etc/resolv.conf
```

---

## Testing Framework

### Unit Testing

#### Rust Tests

```rust
// nodes/agent/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_node_registration() {
        let config = NodeConfig::default();
        let client = DirectoryClient::new(&config).await.unwrap();
        
        let result = client.register_node().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_wireguard_key_generation() {
        let keypair = WireGuardKeypair::generate();
        assert_eq!(keypair.public_key().len(), 44); // Base64 encoded
    }
}
```

#### Python Tests

```python
# services/auth-api/tests/test_auth.py
import pytest
from fastapi.testclient import TestClient
from app.main import app

client = TestClient(app)

class TestAuthentication:
    def test_user_registration(self):
        response = client.post("/users/register", json={
            "email": "test@example.com",
            "password": "SecurePassword123!",
            "consent": True,
            "tos_version": "v1.0"
        })
        assert response.status_code == 201
        assert "user_id" in response.json()

    def test_login_success(self):
        # First register user
        self.test_user_registration()
        
        # Then login
        response = client.post("/auth/login", json={
            "email": "test@example.com",
            "password": "SecurePassword123!"
        })
        assert response.status_code == 200
        assert "access_token" in response.json()
        assert "refresh_token" in response.json()

    def test_login_invalid_credentials(self):
        response = client.post("/auth/login", json={
            "email": "test@example.com",
            "password": "WrongPassword"
        })
        assert response.status_code == 401
```

### Integration Testing

#### End-to-End Test Suite

```python
#!/usr/bin/env python3
# tests/integration/test_e2e_flow.py

import asyncio
import pytest
import httpx
from wireguard import WireGuardKeypair

class TestE2EFlow:
    """End-to-end integration tests"""
    
    @pytest.fixture
    async def auth_client(self):
        async with httpx.AsyncClient(base_url="http://localhost:8080") as client:
            yield client
    
    @pytest.fixture
    async def directory_client(self):
        async with httpx.AsyncClient(base_url="http://localhost:8081") as client:
            yield client
    
    async def test_complete_user_flow(self, auth_client, directory_client):
        """Test complete user registration -> login -> VPN connection flow"""
        
        # 1. Register user
        user_data = {
            "email": "integration@example.com",
            "password": "TestPassword123!",
            "consent": True,
            "tos_version": "v1.0"
        }
        response = await auth_client.post("/users/register", json=user_data)
        assert response.status_code == 201
        user_id = response.json()["user_id"]
        
        # 2. Login and get tokens
        login_data = {
            "email": user_data["email"],
            "password": user_data["password"]
        }
        response = await auth_client.post("/auth/login", json=login_data)
        assert response.status_code == 200
        tokens = response.json()
        access_token = tokens["access_token"]
        
        # 3. Register device
        keypair = WireGuardKeypair.generate()
        device_data = {
            "device_public_key_b64": keypair.public_key_b64(),
            "platform": "linux",
            "device_name": "Test Device"
        }
        headers = {"Authorization": f"Bearer {access_token}"}
        response = await auth_client.post("/devices/register", json=device_data, headers=headers)
        assert response.status_code == 201
        device_id = response.json()["device_id"]
        
        # 4. Get available regions
        response = await directory_client.get("/regions")
        assert response.status_code == 200
        regions = response.json()
        assert len(regions) > 0
        region_id = regions[0]["id"]
        
        # 5. Allocate VPN session
        session_data = {
            "region_id": region_id,
            "mode": "single-hop",
            "client_public_key": keypair.public_key_b64()
        }
        response = await directory_client.post("/mesh/client-config", json=session_data, headers=headers)
        assert response.status_code == 201
        config = response.json()
        
        # 6. Verify WireGuard configuration
        assert "peer_public_key" in config
        assert "endpoint" in config
        assert "allowed_ips" in config
        assert config["client_ip"].startswith("10.66.")
        
        # 7. Test session termination
        session_id = config["session_id"]
        response = await directory_client.delete(f"/mesh/sessions/{session_id}", headers=headers)
        assert response.status_code == 204
```

#### Service Integration Tests

```bash
#!/bin/bash
# tests/integration/run_integration_tests.sh

set -euo pipefail

echo "🧪 Running integration tests..."

# Start test environment
docker compose -f docker-compose.test.yml up -d
sleep 30  # Wait for services to be ready

# Wait for services to be healthy
./scripts/wait_for_services.sh

# Seed test data
python tests/integration/seed_test_data.py

# Run integration tests
pytest tests/integration/ -v --tb=short

# Cleanup
docker compose -f docker-compose.test.yml down -v

echo "✅ Integration tests completed"
```

### Performance Testing

#### Load Testing with Locust

```python
# tests/performance/locustfile.py
from locust import HttpUser, task, between
import random
import string

class VPNUser(HttpUser):
    wait_time = between(1, 3)
    
    def on_start(self):
        """Setup user session"""
        # Register user
        email = f"test{''.join(random.choices(string.ascii_lowercase, k=8))}@example.com"
        password = "TestPassword123!"
        
        response = self.client.post("/users/register", json={
            "email": email,
            "password": password,
            "consent": True,
            "tos_version": "v1.0"
        })
        
        # Login and store tokens
        response = self.client.post("/auth/login", json={
            "email": email,
            "password": password
        })
        
        if response.status_code == 200:
            tokens = response.json()
            self.access_token = tokens["access_token"]
            self.headers = {"Authorization": f"Bearer {self.access_token}"}
    
    @task(3)
    def get_regions(self):
        """Test region listing"""
        self.client.get("/regions", headers=self.headers)
    
    @task(2)
    def refresh_token(self):
        """Test token refresh"""
        self.client.post("/auth/refresh", json={
            "refresh_token": self.refresh_token
        })
    
    @task(1)
    def allocate_session(self):
        """Test VPN session allocation"""
        # Get regions first
        response = self.client.get("/regions", headers=self.headers)
        if response.status_code == 200:
            regions = response.json()
            if regions:
                region_id = random.choice(regions)["id"]
                
                # Allocate session
                self.client.post("/mesh/client-config", json={
                    "region_id": region_id,
                    "mode": "single-hop",
                    "client_public_key": "fake_key_for_testing"
                }, headers=self.headers)
```

#### Benchmark Scripts

```bash
#!/bin/bash
# tests/performance/benchmark.sh

echo "📊 Running performance benchmarks..."

# API endpoint benchmarks
echo "=== API Benchmarks ==="
ab -n 1000 -c 10 http://localhost:8080/healthz
ab -n 500 -c 5 -H "Authorization: Bearer $TEST_TOKEN" http://localhost:8081/regions

# Database performance
echo "=== Database Benchmarks ==="
pgbench -h localhost -p 5432 -U postgres -d postgres -c 10 -j 2 -t 1000

# WireGuard throughput
echo "=== WireGuard Throughput ==="
iperf3 -c 10.66.0.1 -t 30 -P 4

echo "✅ Benchmarks completed"
```

---

## Chaos Engineering

### Chaos Testing Framework

#### Chaos Monkey Implementation

```python
#!/usr/bin/env python3
# tools/chaos/chaos.py

import random
import time
import docker
import requests
from typing import List, Dict

class ChaosMonkey:
    """Chaos engineering tool for VPN infrastructure"""
    
    def __init__(self):
        self.docker_client = docker.from_env()
        self.scenarios = [
            self.kill_random_service,
            self.introduce_network_latency,
            self.fill_disk_space,
            self.consume_memory,
            self.drop_database_connections
        ]
    
    def kill_random_service(self):
        """Randomly kill a service container"""
        services = ['auth-api', 'directory-api', 'admin-api']
        service = random.choice(services)
        
        try:
            container = self.docker_client.containers.get(f"vpn-mvp-{service}-1")
            print(f"🔥 Killing service: {service}")
            container.kill()
            
            # Wait and check if it restarts
            time.sleep(30)
            container.reload()
            if container.status == 'running':
                print(f"✅ Service {service} recovered automatically")
            else:
                print(f"❌ Service {service} did not recover")
                
        except docker.errors.NotFound:
            print(f"Service {service} not found")
    
    def introduce_network_latency(self):
        """Add network latency using tc (traffic control)"""
        print("🐌 Introducing network latency...")
        
        # Add 100ms latency to all traffic
        self.docker_client.containers.run(
            "nicolaka/netshoot",
            command="tc qdisc add dev eth0 root netem delay 100ms",
            network_mode="container:vpn-mvp-auth-api-1",
            privileged=True,
            remove=True
        )
        
        time.sleep(60)  # Let it run for 1 minute
        
        # Remove latency
        self.docker_client.containers.run(
            "nicolaka/netshoot", 
            command="tc qdisc del dev eth0 root",
            network_mode="container:vpn-mvp-auth-api-1",
            privileged=True,
            remove=True
        )
        print("✅ Network latency removed")
    
    def fill_disk_space(self):
        """Fill up disk space to test disk pressure"""
        print("💾 Filling disk space...")
        
        # Create large file to consume disk space
        self.docker_client.containers.run(
            "alpine",
            command="dd if=/dev/zero of=/tmp/largefile bs=1M count=1000",
            volumes={'/tmp': {'bind': '/tmp', 'mode': 'rw'}},
            remove=True
        )
        
        time.sleep(30)
        
        # Cleanup
        self.docker_client.containers.run(
            "alpine",
            command="rm -f /tmp/largefile",
            volumes={'/tmp': {'bind': '/tmp', 'mode': 'rw'}},
            remove=True
        )
        print("✅ Disk space restored")
    
    def run_scenario(self, scenario_name: str = None):
        """Run a specific chaos scenario or random one"""
        if scenario_name:
            scenario = getattr(self, scenario_name, None)
            if scenario:
                scenario()
            else:
                print(f"Unknown scenario: {scenario_name}")
        else:
            scenario = random.choice(self.scenarios)
            scenario()
    
    def run_continuous(self, duration_minutes: int = 60):
        """Run chaos scenarios continuously"""
        end_time = time.time() + (duration_minutes * 60)
        
        print(f"🎭 Starting chaos testing for {duration_minutes} minutes...")
        
        while time.time() < end_time:
            self.run_scenario()
            
            # Wait between scenarios
            wait_time = random.randint(30, 300)  # 30s to 5min
            print(f"⏳ Waiting {wait_time}s before next scenario...")
            time.sleep(wait_time)
        
        print("✅ Chaos testing completed")

if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="VPN Chaos Engineering Tool")
    parser.add_argument("--scenario", help="Specific scenario to run")
    parser.add_argument("--duration", type=int, default=60, help="Duration in minutes")
    parser.add_argument("--continuous", action="store_true", help="Run continuously")
    
    args = parser.parse_args()
    
    chaos = ChaosMonkey()
    
    if args.continuous:
        chaos.run_continuous(args.duration)
    else:
        chaos.run_scenario(args.scenario)
```

#### Chaos Testing Scenarios

```bash
#!/bin/bash
# tools/chaos/scenarios/network_partition.sh

echo "🌐 Simulating network partition..."

# Block traffic between auth-api and database
docker exec vpn-mvp-auth-api-1 iptables -A OUTPUT -d postgres -j DROP

echo "Network partition active for 2 minutes..."
sleep 120

# Restore connectivity
docker exec vpn-mvp-auth-api-1 iptables -D OUTPUT -d postgres -j DROP

echo "✅ Network partition resolved"
```

---

## Security Tools

### PKI Management Tools

#### Certificate Authority Setup

```python
#!/usr/bin/env python3
# tools/pki/gen_dev_certs.py

import os
import subprocess
from pathlib import Path
from cryptography import x509
from cryptography.x509.oid import NameOID
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
import datetime

class PKIManager:
    """Development PKI certificate management"""
    
    def __init__(self, base_path: str = "tools/pki/dev"):
        self.base_path = Path(base_path)
        self.base_path.mkdir(parents=True, exist_ok=True)
    
    def generate_ca_certificate(self):
        """Generate root CA certificate for development"""
        print("🔐 Generating root CA certificate...")
        
        # Generate private key
        private_key = rsa.generate_private_key(
            public_exponent=65537,
            key_size=4096
        )
        
        # Create certificate
        subject = issuer = x509.Name([
            x509.NameAttribute(NameOID.COUNTRY_NAME, "US"),
            x509.NameAttribute(NameOID.STATE_OR_PROVINCE_NAME, "CA"),
            x509.NameAttribute(NameOID.LOCALITY_NAME, "San Francisco"),
            x509.NameAttribute(NameOID.ORGANIZATION_NAME, "VPN Dev CA"),
            x509.NameAttribute(NameOID.COMMON_NAME, "VPN Development Root CA"),
        ])
        
        cert = x509.CertificateBuilder().subject_name(
            subject
        ).issuer_name(
            issuer
        ).public_key(
            private_key.public_key()
        ).serial_number(
            x509.random_serial_number()
        ).not_valid_before(
            datetime.datetime.utcnow()
        ).not_valid_after(
            datetime.datetime.utcnow() + datetime.timedelta(days=3650)
        ).add_extension(
            x509.BasicConstraints(ca=True, path_length=None),
            critical=True,
        ).add_extension(
            x509.KeyUsage(
                key_cert_sign=True,
                crl_sign=True,
                digital_signature=False,
                key_encipherment=False,
                key_agreement=False,
                content_commitment=False,
                data_encipherment=False,
                encipher_only=False,
                decipher_only=False
            ),
            critical=True,
        ).sign(private_key, hashes.SHA256())
        
        # Save certificate and key
        ca_cert_path = self.base_path / "ca.crt"
        ca_key_path = self.base_path / "ca.key"
        
        with open(ca_cert_path, "wb") as f:
            f.write(cert.public_bytes(serialization.Encoding.PEM))
        
        with open(ca_key_path, "wb") as f:
            f.write(private_key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.PKCS8,
                encryption_algorithm=serialization.NoEncryption()
            ))
        
        os.chmod(ca_key_path, 0o600)
        print(f"✅ CA certificate saved to {ca_cert_path}")
        return cert, private_key
    
    def generate_service_certificate(self, service_name: str, ca_cert, ca_key):
        """Generate service certificate signed by CA"""
        print(f"🔐 Generating certificate for {service_name}...")
        
        # Generate private key
        private_key = rsa.generate_private_key(
            public_exponent=65537,
            key_size=2048
        )
        
        # Create certificate
        subject = x509.Name([
            x509.NameAttribute(NameOID.COUNTRY_NAME, "US"),
            x509.NameAttribute(NameOID.STATE_OR_PROVINCE_NAME, "CA"),
            x509.NameAttribute(NameOID.LOCALITY_NAME, "San Francisco"),
            x509.NameAttribute(NameOID.ORGANIZATION_NAME, "VPN Service"),
            x509.NameAttribute(NameOID.COMMON_NAME, f"{service_name}.vpn.local"),
        ])
        
        cert = x509.CertificateBuilder().subject_name(
            subject
        ).issuer_name(
            ca_cert.subject
        ).public_key(
            private_key.public_key()
        ).serial_number(
            x509.random_serial_number()
        ).not_valid_before(
            datetime.datetime.utcnow()
        ).not_valid_after(
            datetime.datetime.utcnow() + datetime.timedelta(days=30)
        ).add_extension(
            x509.SubjectAlternativeName([
                x509.DNSName(f"{service_name}.vpn.local"),
                x509.DNSName(f"{service_name}"),
                x509.DNSName("localhost"),
                x509.IPAddress(ipaddress.IPv4Address("127.0.0.1")),
            ]),
            critical=False,
        ).add_extension(
            x509.KeyUsage(
                key_cert_sign=False,
                crl_sign=False,
                digital_signature=True,
                key_encipherment=True,
                key_agreement=False,
                content_commitment=False,
                data_encipherment=False,
                encipher_only=False,
                decipher_only=False
            ),
            critical=True,
        ).sign(ca_key, hashes.SHA256())
        
        # Save certificate and key
        cert_path = self.base_path / f"{service_name}.crt"
        key_path = self.base_path / f"{service_name}.key"
        
        with open(cert_path, "wb") as f:
            f.write(cert.public_bytes(serialization.Encoding.PEM))
        
        with open(key_path, "wb") as f:
            f.write(private_key.private_bytes(
                encoding=serialization.Encoding.PEM,
                format=serialization.PrivateFormat.PKCS8,
                encryption_algorithm=serialization.NoEncryption()
            ))
        
        os.chmod(key_path, 0o600)
        print(f"✅ Service certificate saved to {cert_path}")

if __name__ == "__main__":
    pki = PKIManager()
    
    # Generate CA
    ca_cert, ca_key = pki.generate_ca_certificate()
    
    # Generate service certificates
    services = ["auth-api", "directory-api", "admin-api"]
    for service in services:
        pki.generate_service_certificate(service, ca_cert, ca_key)
    
    print("🎉 Development certificates generated successfully!")
```

### Vulnerability Scanning

#### Container Security Scanning

```bash
#!/bin/bash
# tools/security/scan_images.sh

set -euo pipefail

echo "🔍 Scanning container images for vulnerabilities..."

IMAGES=(
    "vpn/auth-api:latest"
    "vpn/directory-api:latest" 
    "vpn/admin-api:latest"
    "vpn/node-agent:latest"
)

# Install trivy if not present
if ! command -v trivy &> /dev/null; then
    echo "Installing Trivy..."
    curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin
fi

for image in "${IMAGES[@]}"; do
    echo "Scanning $image..."
    
    # Scan for vulnerabilities
    trivy image --severity HIGH,CRITICAL --format json --output "security-scan-$(basename $image).json" $image
    
    # Generate human-readable report
    trivy image --severity HIGH,CRITICAL $image
    
    echo "---"
done

echo "✅ Security scanning completed"
```

#### Dependency Scanning

```bash
#!/bin/bash
# tools/security/scan_dependencies.sh

echo "📦 Scanning dependencies for vulnerabilities..."

# Rust dependencies
echo "=== Rust Dependencies ==="
cargo audit

# Python dependencies  
echo "=== Python Dependencies ==="
for service in auth-api directory-api admin-api; do
    echo "Scanning services/$service..."
    cd "services/$service"
    safety check -r requirements.txt
    cd ../..
done

# Node.js dependencies
echo "=== Node.js Dependencies ==="
cd clients/desktop/ui
npm audit
cd ../../..

echo "✅ Dependency scanning completed"
```

---

## Quality Assurance Tools

### Code Formatting and Linting

#### Pre-commit Configuration

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
      - id: check-merge-conflict

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

#### Rust Formatting and Linting

```toml
# rustfmt.toml
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
edition = "2021"
```

```toml
# clippy.toml
cognitive-complexity-threshold = 30
too-many-arguments-threshold = 8
type-complexity-threshold = 250
single-char-lifetime-names-threshold = 4
trivial-copy-size-limit = 64
```

### Static Analysis

#### SonarQube Configuration

```properties
# sonar-project.properties
sonar.projectKey=vpn-mvp
sonar.projectName=VPN MVP
sonar.projectVersion=1.0

# Source directories
sonar.sources=services,nodes,clients
sonar.exclusions=**/target/**,**/node_modules/**,**/__pycache__/**

# Language-specific settings
sonar.python.coverage.reportPaths=coverage.xml
sonar.rust.clippy.reportPaths=clippy-report.json

# Quality gates
sonar.qualitygate.wait=true
```

#### Code Coverage

```bash
#!/bin/bash
# tools/quality/coverage.sh

echo "📊 Generating code coverage reports..."

# Rust coverage
echo "=== Rust Coverage ==="
cargo tarpaulin --out xml --output-dir target/coverage

# Python coverage
echo "=== Python Coverage ==="
for service in auth-api directory-api admin-api; do
    cd "services/$service"
    coverage run -m pytest
    coverage xml -o ../../target/coverage/coverage-$service.xml
    cd ../..
done

# Combine coverage reports
coverage combine target/coverage/coverage-*.xml

echo "✅ Coverage reports generated in target/coverage/"
```

---

## Automation and CI/CD Tools

### GitHub Actions Workflows

#### Main CI Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test-rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: rustfmt, clippy
          
      - name: Cache cargo
        uses: actions/cache@v3
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
          
      - name: Format check
        run: cargo fmt --all -- --check
        
      - name: Lint
        run: cargo clippy --workspace -- -D warnings
        
      - name: Test
        run: cargo test --workspace

  test-python:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        service: [auth-api, directory-api, admin-api]
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'
          
      - name: Install dependencies
        run: |
          cd services/${{ matrix.service }}
          pip install -r requirements.txt
          pip install pytest coverage
          
      - name: Lint
        run: |
          cd services/${{ matrix.service }}
          ruff check .
          
      - name: Test
        run: |
          cd services/${{ matrix.service }}
          coverage run -m pytest
          coverage xml

  integration-test:
    runs-on: ubuntu-latest
    needs: [test-rust, test-python]
    steps:
      - uses: actions/checkout@v3
      
      - name: Start services
        run: docker compose up -d
        
      - name: Wait for services
        run: ./scripts/wait_for_services.sh
        
      - name: Run integration tests
        run: pytest tests/integration/ -v
        
      - name: Cleanup
        run: docker compose down -v

  security-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Run Trivy vulnerability scanner
        uses: aquasecurity/trivy-action@master
        with:
          scan-type: 'fs'
          scan-ref: '.'
          format: 'sarif'
          output: 'trivy-results.sarif'
          
      - name: Upload Trivy scan results
        uses: github/codeql-action/upload-sarif@v2
        with:
          sarif_file: 'trivy-results.sarif'
```

### Release Automation

#### Release Script

```bash
#!/bin/bash
# scripts/release.sh

set -euo pipefail

VERSION=${1:-}
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 v1.2.3"
    exit 1
fi

echo "🚀 Preparing release $VERSION..."

# Validate version format
if [[ ! $VERSION =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in format vX.Y.Z"
    exit 1
fi

# Check if working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    echo "Error: Working directory is not clean"
    exit 1
fi

# Run tests
echo "Running tests..."
make test

# Update version in files
echo "Updating version numbers..."
sed -i "s/version = \".*\"/version = \"${VERSION#v}\"/" Cargo.toml
sed -i "s/\"version\": \".*\"/\"version\": \"${VERSION#v}\"/" clients/desktop/ui/package.json

# Build release artifacts
echo "Building release artifacts..."
make build

# Generate changelog
echo "Generating changelog..."
git log --oneline --pretty=format:"- %s" $(git describe --tags --abbrev=0)..HEAD > CHANGELOG-$VERSION.md

# Create git tag
echo "Creating git tag..."
git add -A
git commit -m "Release $VERSION"
git tag -a $VERSION -m "Release $VERSION"

# Push to remote
echo "Pushing to remote..."
git push origin main
git push origin $VERSION

echo "✅ Release $VERSION prepared successfully!"
echo "GitHub Actions will handle building and publishing artifacts."
```

---

For more detailed information on specific tools and workflows, see:
- [Development Setup Guide](../../CONTRIBUTING.md)
- [Testing Documentation](../../TESTING.md)
- [Security Documentation](../security/)
- [Operations Guide](../ops/INDEX.md)
