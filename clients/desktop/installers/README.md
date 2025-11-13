Installers per OS (Windows MSI/MSIX, macOS DMG/notarization, Linux DEB/RPM/AppImage).

This directory contains **packaging scaffolds** for the desktop client:

- `windows/`: MSI/MSIX packaging for the Tauri UI + `desktop-service`.
- `macos/`: universal app bundle + DMG, with hooks for notarization.
- `linux/`: DEB/RPM/AppImage packaging using the Tauri bundler.

Use these as reference entry points for CI pipelines; they do not replace the
Tauri bundler configuration in `clients/desktop/ui/src-tauri/tauri.conf.json`.

