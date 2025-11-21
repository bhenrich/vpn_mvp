## Data Processing Addendum (DPA)

This DPA forms part of the agreement between Controller (customer) and Processor (this service) and reflects the parties’ agreement with regard to the processing of personal data.

Scope:
- Purpose: Provide VPN authentication, device management, and region directory services.
- Nature: Authentication and authorization; minimal account data storage.
- Duration: For the term of the service, or until deletion request completion.

Data categories:
- Users: email address, password hash (Argon2id), consent flags, created_at.
- Devices: device public key, platform, timestamps.
- Sessions: bounded-lifetime session metadata (JTI references), expiry timestamps.
- No traffic data, destination IPs, or DNS queries are stored.

Subprocessors:
- Postgres (managed by operator), Redis (managed by operator), container registry and CI systems for builds.

Security measures:
- Strong password hashing (Argon2id), JWT RS256 with rotating keys (JWKS), mTLS for control-plane, RAM-only node state.
- No access logs (disabled at ingress layer and app servers), structured logs with no PII.
- Data-at-rest encryption recommended for Postgres volumes; TLS in transit.

Data subject rights:
- Export: `/gdpr/export` returns all user-related data (no traffic logs exist by design).
- Erasure: `/gdpr/delete` hard-deletes the account and all dependent rows.
- Consent: consent and residency flags recorded at registration; updates available via account flows.

International transfers and residency:
- Regions and residency are enforced by directory selection; operator config ensures EU-only processing for EU residents where required.

Breach notification:
- Processor will notify Controller without undue delay upon becoming aware of a personal data breach and provide available information for Controller disclosure obligations.

Retention:
- Minimal operational data only; no traffic logs; session data only for token validity windows; audit entries are redacted and non-identifying.

Instructions:
- Processor processes personal data solely on Controller’s documented instructions as embodied in API usage and configuration.


