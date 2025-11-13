Windows installers (MSI/MSIX)
=============================

This directory contains scaffolding for Windows packaging of the desktop client
(`vpn-gui` / Tauri UI) plus the background `desktop-service`.

The recommended approach is:

- Use the Tauri bundler to produce a signed Windows app bundle.
- Wrap the resulting binaries into an MSI/MSIX using WiX Toolset or MSIX tooling.

The helper script `build.ps1` assumes:

- Node + pnpm are installed.
- Rust toolchain with `x86_64-pc-windows-msvc` target is installed.
- WiX/MSIX toolchain is configured in the build environment.


