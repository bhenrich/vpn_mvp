## Records of Processing Activities (ROPA)

Controller: Customer organization using the VPN service.
Processor: This VPN control-plane and clients.

Processing purposes:
- Authenticate users and devices.
- Authorize client connections to nodes and enforce active-connection limits.
- Provide region directory and node health/topology to clients.

Categories of data subjects:
- End users of the VPN service.

Categories of personal data:
- Identification: email address.
- Authentication: password hash (Argon2id), optional TOTP secret.
- Devices: device public keys, platform.
- Session: JWT JTIs, expiry timestamps.
- No traffic data (websites, destination IPs, DNS queries).

Recipients:
- None, except infrastructure necessary to run the service (DB, cache, CI/CD).

Third-country transfers:
- Controlled by region and residency configuration; EU residency can be enforced.

Retention periods:
- User account data retained until deletion request.
- Sessions retained only for validity windows; refresh token JTIs expire in <= 7 days.
- Logs contain no PII; metrics are aggregate only.

Security measures:
- RS256 JWT with regular key rotation and JWKS exposure.
- TLS/mTLS for service communication.
- RAM-only node operation; iptables/nftables kill switch; DNS leak prevention.

Data subject rights:
- Export and erasure implemented as API endpoints; documented in `docs/`.


