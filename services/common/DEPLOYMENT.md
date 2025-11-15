## services/common – Deployment Guide

**Role**: Internal shared Python package (schemas, protobufs, observability) for control‑plane services.

`services/common` is not deployed as a standalone service. Instead, it is bundled into the images of `auth-api`, `directory-api`, `admin-api`, and any other control‑plane services.

### How it is used in deployment

- Docker build contexts for services include the `services/common` package.
- When you build a service image (e.g. `auth-api`), its Dockerfile installs this package (either via a relative path, local wheel, or workspace‑style layout).

No separate runtime configuration is required for `services/common`; deployment concerns are entirely handled by the consuming services.

### Updating shared contracts

When you change:

- Protobuf definitions under `protos/`.
- Shared observability behaviour.
- Shared pydantic models or OpenAPI fragments.

You should:

1. Regenerate any derived code (e.g. gRPC stubs).
2. Rebuild the consumer service images (e.g. `docker compose build auth-api directory-api admin-api`).
3. Roll out updated images in your orchestrator with appropriate compatibility testing.


