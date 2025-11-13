Core Rust networking engine (WireGuard via `wireguard-rs`) for desktop clients.

This crate (`desktop-core`) wraps the lower-level `vpn-core` and `platform-*`
crates to provide a high-level, cross-platform API for:

- TUN lifecycle (`PlatformTun`) using per-OS backends (`wintun`, `utun`, `/dev/net/tun`).
- Kill switch management (`KillSwitch`) using native firewalls (WFP, PF, nftables).
- DNS override and leak protection (`DnsOverride`).
- Route-based split tunnelling (`SplitTunnelRoutes`).
- Auto-connect background loops (`spawn_autoconnect`) with trusted SSIDs and MTU tuning.

It is designed to be consumed by:

- The background desktop service (`desktop-service`) to manage connectivity.
- The Tauri UI (`clients/desktop/ui`) via a thin IPC layer.

