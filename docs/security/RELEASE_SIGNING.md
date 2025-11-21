## Release Signing and Updater Verification

This repository ships:
- Signed update artifacts via Tauri updater (Ed25519).
- SBOMs for Python and Rust.
- Cosign signatures for release binaries.

Secrets required (GitHub Actions):
- `TAURI_PRIVATE_KEY`: Base64-encoded Tauri Ed25519 private key.
- `TAURI_KEY_PASSWORD`: Password protecting the Tauri private key.
- `TAURI_UPDATER_PUBKEY`: Base64 public key (Ed25519) for client update verification.

Workflows:
- `.github/workflows/tauri-release.yml`:
  - Injects updater `pubkey` and endpoint into `clients/desktop/ui/tauri.conf.json` at build time.
  - Builds platform installers.
  - Publishes release and update manifest (`latest.json`).
  - Signs release artifacts using Sigstore Cosign (keyless).
- `.github/workflows/supply-chain.yml`:
  - Generates CycloneDX SBOMs for Python and Rust.
  - Signs SBOMs with Cosign and uploads them as artifacts.

Client verification:
- The Tauri updater in production builds is set to `active=true`, `pubkey` = `TAURI_UPDATER_PUBKEY`.
- Clients verify update signatures before installing; mismatches are rejected.


