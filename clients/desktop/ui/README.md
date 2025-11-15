# Linux Dependencies for Tauri Client

## Arch Linux

Install the required system dependencies:

```bash
sudo pacman -Syu webkit2gtk libayatana-appindicator gtk3 openssl
```

### Package Details:
- **webkit2gtk**: Provides JavaScriptCore GTK library (required for webview)
- **libayatana-appindicator**: System tray support (optional but recommended)
- **gtk3**: GTK+ 3 toolkit (usually already installed)
- **openssl**: SSL/TLS library (usually already installed)

## Other Linux Distributions

### Debian/Ubuntu:
```bash
sudo apt-get update
sudo apt-get install libwebkit2gtk-4.0-dev libayatana-appindicator3-dev libssl-dev libgtk-3-dev
```

### Fedora:
```bash
sudo dnf install webkit2gtk3-devel.x86_64 libappindicator-gtk3 openssl-devel gtk3-devel
```

### OpenSUSE:
```bash
sudo zypper install webkit2gtk3-devel libappindicator3 gtk3-devel libopenssl-devel
```

After installing dependencies, you should be able to run:
```bash
pnpm tauri:dev
```

