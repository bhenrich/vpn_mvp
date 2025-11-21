## External Penetration Test Coordination Plan

Objectives:
- Validate the security posture of control-plane services (auth, directory, admin) and node agent interfaces.
- Verify there are no PII leakages, excessive logging, or insecure defaults.
- Assess JWT/JWKS rotation robustness and mTLS certificate handling.

Scope:
- In-scope: HTTPS APIs for `auth-api`, `directory-api`, `admin-api`; gRPC endpoints exposed by directory; container images and Dockerfiles; SBOMs; update pipelines.
- Out-of-scope: Customer-operated perimeter (WAF, rate limiting), production infrastructure specifics not represented in this repo.

Constraints:
- Test environment must not contain real user data; only seeded test accounts.
- Logs must remain PII-free; access logs are disabled by default.

Methodology:
- Black-box API testing with OpenAPI definitions.
- JWT integrity, audience/issuer ignoring tests, expired token replay, refresh token rotation enforcement.
- mTLS misconfiguration attempts, certificate spoofing attempts against node bootstrap.
- Container image scanning against SBOMs and base images.
- Privilege escalation attempts within containers; filesystem and capability review.

Deliverables:
- Findings report with CVSS scoring, proof-of-concept where applicable, remediation recommendations.
- Verification report post-fix retest.

Operational Steps:
1. Provision dedicated test environment using root `docker-compose.yml`.
2. Seed test data; enable test-only endpoints if required (none by default).
3. Provide testers with JWKS endpoint, sample JWTs, and gRPC reflection endpoints if enabled.
4. Collect findings; triage; fix; retest.


