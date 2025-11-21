# Contributing
## Quick Start

1. **Install prerequisites:**  
   - Docker & Docker Compose  
   - Rust (stable)  
   - Python 3.12+

2. **Clone the repository and bootstrap:**  
   - Run `make bootstrap` to set up tools and hooks.

3. **Service environment:**  
   - Copy each `env.example` to `.env` before running services.
   - Example:  
     ```
     cp services/auth-api/env.example services/auth-api/.env
     ```

## Useful Commands

```bash
make build     # Build all containers
make up        # Start the dev stack
make down      # Stop and remove dev stack
make fmt       # Format Rust code
make lint      # Run Python and Rust linters
make test      # Run Python and Rust tests
```

## Docs
Full system, compliance, and security documentation lives in the [`docs/`](docs/INDEX.md) directory.  
- Architecture, onboarding, and operational guides: [`docs/INDEX.md`](docs/INDEX.md)
- Data Processing Addendum: [`docs/compliance/DPA.md`](docs/compliance/DPA.md)
- Records of Processing Activities: [`docs/compliance/ROPA.md`](docs/compliance/ROPA.md)
- Security threat model: [`docs/security/SECURITY_THREAT_MODEL.md`](docs/security/SECURITY_THREAT_MODEL.md)

Please read relevant docs before submitting changes affecting compliance, data handling, or security-critical flows.


## Notes

- The legacy `server/` Go/OpenVPN implementation is deprecated.
- Current stack: Python (FastAPI) for control plane, Rust/WireGuard for data plane.
- Please document significant changes and follow code style with provided formatters/linters.

