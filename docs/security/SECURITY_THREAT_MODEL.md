## Security Threat Model (MVP)

Assets:
- User identities (emails), device public keys, authentication secrets, JWKS private keys.
- Node private keys (ephemeral, RAM-only), control-plane credentials and certificates.

Trust boundaries:
- Clients on untrusted networks.
- Control-plane services (auth, directory, admin).
- Node plane (RAM-only agents).
- CI/CD and signing infrastructure.

Threats and mitigations:
- Credential stuffing: Argon2id, TOTP option, rate limiting at edge (out of scope here).
- Token theft/replay: Short-lived access tokens (15m), rotating refresh tokens bound to JTI in Redis, JWKS rotation.
- Key compromise (server): Frequent RS256 key rotation; JWKS caches limited by refresh TTL; secrets not logged.
- Traffic deanonymization: No traffic logging; DNS leak protection; kill switch enforced per OS.
- PII in logs: Access logs disabled; application logs exclude request bodies and PII fields.
- Supply chain: SBOMs for Python/Rust; signed artifacts; Tauri update signature verification.
- Data residency: Region selection and operator policy to keep EU data in-region.
- Node compromise: RAM-only state; key rotation on reboot/daily; minimal privileges in containers.

Assumptions:
- Perimeter rate-limiting, DoS protections, and WAF are operated outside this repo.
- Secrets distribution and HSM/KMS for CA hierarchy to be integrated in production.

Open items:
- Formal mTLS CA hierarchy documents and automation track (see DPA).
- External pen-test before GA; scope includes control-plane APIs and node agent.


