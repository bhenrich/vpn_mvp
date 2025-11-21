## PKI - CA Hierarchy and Key Rotation

Goal: Establish offline Root CA and online (short-lived) Intermediate CA for issuing mTLS service certificates and node agent certs. Keys are stored securely (Root offline; Intermediate on hardened host or HSM/KMS).

Hierarchy:
- Root CA (offline, 10y): used only to sign Intermediate CA CSRs.
- Intermediate CA (online, 90d–180d): used to sign service and node certificates (<= 30d).

Rotation policy:
- Intermediate CA rotates every 90 days (overlap window 14 days).
- Service/node leaf certs rotate every 7–30 days depending on role.
- JWKS keys (for JWT) rotate every 24 hours (separate from mTLS PKI).

Artifacts:
- Root: `rootCA.key`, `rootCA.crt` kept offline.
- Intermediate: `intermediate.key`, `intermediate.crt`, chain `fullchain.crt`.

Automation:
- See `tools/pki/init_ca.sh` to initialize a Root+Intermediate.
- See `tools/pki/rotate_intermediate.sh` to rotate Intermediate and update `current/` symlink.
- CI/CD should distribute `fullchain.crt` and `intermediate.key` to secret managers or HSM-backed agents; never commit private keys.

Operational guidance:
- Protect keys using OS permissions; prefer HSM/KMS where available.
- Maintain CRLs/OCSP for revocation; endpoints served privately for control-plane validation.
- Document issuance and revocation events; keep audit trail without PII.


