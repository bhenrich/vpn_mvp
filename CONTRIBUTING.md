Contributing Guidelines
=======================

Thanks for contributing! This repository is being refactored into a monorepo per `PLAN.md`.

Development setup
-----------------
- Install Docker and Docker Compose
- Rust (stable), Python 3.12

Common commands
---------------

```bash
make build     # build all containers
make up        # launch dev stack
make down      # stop and remove dev stack
make fmt       # rustfmt
make lint      # ruff + clippy
make test      # pytest + cargo test
```

Service environment
-------------------
Each service provides an `env.example`. Copy to `.env` before running locally:

```bash
cp services/auth-api/env.example services/auth-api/.env
cp services/directory-api/env.example services/directory-api/.env
cp services/admin-api/env.example services/admin-api/.env
```

Legacy components
-----------------
The previous Go/OpenVPN stack under `server/` is deprecated and will be archived. The new stack uses Python (FastAPI) for the control plane and Rust (`wireguard-rs`) for the data plane.


