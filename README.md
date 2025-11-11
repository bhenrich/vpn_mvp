Rust IKEv2 VPN Client (GUI MVP)
================================

User-mode, cross-platform VPN client GUI in Rust, leveraging OS IKEv2 stacks. MVP implements EAP‑MSCHAPv2 with a basic Iced UI. No custom drivers.

Features
--------
- User-mode only, uses OS IKEv2/IPsec stacks
- Platforms: Windows, macOS, Linux
- Auth: EAP‑MSCHAPv2 (EAP‑TLS types behind `eap-tls` feature, not implemented)
- Reusable core (`vpn-core`) with platform adapters
- Iced GUI app with basic profile management and connect/disconnect

Workspace
---------
- `client/vpn-core`: traits, models, error types, command execution, keyring credentials, file profile store
- `client/platform-windows`: Windows adapter (Add-VpnConnection, rasdial)
- `client/platform-macos`: macOS adapter (scutil --nc)
- `client/platform-linux`: Linux adapters (nmcli, swanctl)
- `client/vpn-gui`: Iced GUI app (MVP)

Prerequisites
-------------
- Windows: PowerShell, `rasdial`, IKEv2 enabled (built-in)
- macOS: `scutil`, IKEv2 supported (built-in)
- Linux:
  - Prefer NetworkManager with strongSwan plugin (`nmcli`)
  - Optional fallback: strongSwan `swanctl`

Build
-----
```bash
cargo build --workspace
```

Run GUI
-------
```bash
cargo run -p vpn-gui
```

Notes
-----
- Some provisioning actions may require admin privileges on Windows/macOS/Linux. The app persists connection profiles in a per-user store and delegates system provisioning to the platform adapters.
- Linux provisioning for IKEv2 varies by distribution. This MVP focuses on `nmcli` and `swanctl` command invocation for existing profiles; robust import/provision flows can be added later.
- EAP‑TLS is planned. The core exposes types and traits behind the `eap-tls` feature flag.


