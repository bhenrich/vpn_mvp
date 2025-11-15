## services/common – Development Guide

**Role**: Shared schemas, protobuf definitions, and observability helpers for control‑plane services.

This package is imported by `auth-api`, `directory-api`, `admin-api`, and potentially other internal components.

### Layout

- `protos/directory.proto` – gRPC protobuf definitions for node/control‑plane communication.
- `observability/` – helpers for wiring metrics/traces into FastAPI apps.

### Prerequisites

- **Runtime**: Python 3.11+.
- This package is usually installed/used via editable installs in the consuming services’ environments.

### Local development

When working on services that depend on `services/common`, the simplest approach is to install it in editable mode into that service’s virtualenv:

```bash
cd services/auth-api          # or directory-api/admin-api
python -m venv .venv
source .venv/bin/activate     # Windows: .venv\Scripts\activate
pip install -r requirements.txt
pip install -e ../common      # editable install of shared package
```

After that, changes under `services/common` will be picked up without reinstalling the package.

### Regenerating protobuf stubs

If you modify `services/common/protos/directory.proto`, you’ll need to regenerate any generated stubs used by services (e.g. under `services/directory-api/app/_gen/`).

From the repo root, a typical pattern is:

```bash
python -m grpc_tools.protoc \
  -I services/common/protos \
  --python_out=services/directory-api/app/_gen \
  --grpc_python_out=services/directory-api/app/_gen \
  services/common/protos/directory.proto
```

Adjust paths/output modules as the codebase evolves.

### Testing

No standalone tests are defined here yet; behaviour is exercised via the services that import this package.  
When adding behaviour to `services/common`, prefer to add tests in the primary consumer service until a dedicated test suite is introduced.


