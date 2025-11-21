# Quick Start - Desktop UI (Tauri)

## Prerequisites

1. **Node.js & pnpm**
   - Install Node.js LTS: `winget install OpenJS.NodeJS.LTS`
   - Enable pnpm: `corepack enable`
   - Prepare pnpm: `corepack prepare pnpm@latest --activate`

2. **Rust** (required for Tauri)
   - Install Rust: `winget install Rustlang.Rustup`
   - Set default toolchain: `rustup default stable`

3. **Visual Studio Build Tools** (Windows only)
   - Install "Desktop development with C++" workload
   - Or use Visual Studio Community with C++ tools

## Development Setup

1. **Install dependencies**
   ```powershell
   cd clients\desktop\ui
   pnpm install
   ```

2. **Start Tauri dev mode**
   ```powershell
   pnpm tauri:dev
   ```

   This will:
   - Start Vite dev server on http://localhost:1420
   - Build and launch the Tauri app window
   - Enable hot-reload for both frontend and Rust code

3. **Build for production**
   ```powershell
   pnpm tauri:build
   ```

## Testing with Local Node Containers

1. **Add local host overrides**

   Point the dev hostnames used by the node containers at the IP of the machine running Docker.
   On Windows, edit `C:\Windows\System32\drivers\etc\hosts` (run your editor as Administrator) and add:

   ```
   192.168.0.183   de-berlin.dev.local
   192.168.0.183   de-munich.dev.local
   ```

   Replace `192.168.0.183` with the LAN IP of your desktop that runs Docker.

2. **Start the control plane and nodes**

   ```powershell
   # From repo root
   docker compose up -d postgres redis auth-api directory-api admin-api directory-seed `
     node-agent-de-berlin node-agent-de-munich
   docker compose logs --tail=50 node-agent-de-berlin
   docker compose logs --tail=50 node-agent-de-munich
   ```

   The `directory-seed` helper ensures the `DE/Berlin` and `DE/Munich` regions exist. Each node
   self-registers, advertises its endpoint (`de-*.dev.local:5182x`), and starts streaming peer updates.

3. **Login and connect from the UI**

   - Launch the Tauri app (`pnpm tauri:dev`).
   - **First time setup**: If prompted, review and accept the telemetry consent.
   - **Server configuration** (if needed): If the backend is running on a different machine, enter the IP address in the "Server address" field and click "Save server".
   - **Login**: Sign in using either:
     - **Direct login**: Enter your email, password, and optional TOTP code, then click "Sign in".
     - **Device login**: Click "Start device login", enter your email hint, then visit the device authorization page on another device and enter the displayed code. The app will automatically check for authorization.
   - **Select region**: The Regions screen should list **Germany • Berlin** and **Germany • Munich**. Select a region and connection mode (Single-server or Multi-hop), then click "Save".
   - **Connect**: Click the "Connect" button. The app will:
     - Register your device (WireGuard keypair stored locally, private key never leaves the client).
     - Request a client session from directory-api (which assigns a `10.66.0.x` IP and pushes the peer to the node).
     - Configure the local WireGuard interface, DNS, and kill switch rules automatically.
   - **Status**: The connection status will update automatically. Green "Connected" indicates a successful connection.

4. **Verify traffic**

   ```powershell
   ping 10.66.0.1
   nslookup example.com 10.66.0.1
   curl -4 https://ifconfig.co
   ```

## Troubleshooting

### Build and Setup Issues

- **"tauri is not recognized"**: Run `pnpm install` first to install `@tauri-apps/cli`
- **Build errors**: Ensure Visual Studio Build Tools with C++ workload is installed
- **Port 1420 in use**: Change port in `vite.config.ts` or kill the process using it

### Connection Issues

- **"Login failed"**: 
  - Verify the server address is correct (if using a remote backend)
  - Check that `auth-api` is running: `docker compose ps auth-api`
  - Ensure your credentials are correct
  - Check for error messages displayed in red below the login form

- **"Failed to connect"**:
  - Verify that `directory-api` and the selected node are running
  - Check that the node has registered: `docker compose logs node-agent-de-berlin`
  - Ensure the hostname/IP in your hosts file matches the node's `PUBLIC_ENDPOINT`
  - Check for error messages displayed in red on the status page

- **"Select a region before connecting"**:
  - Make sure you've selected a region and clicked "Save" before attempting to connect

- **Connection status shows "Disconnected"**:
  - Check `docker compose logs node-agent` for errors
  - Verify the WireGuard interface is up on your system
  - Try disconnecting and reconnecting

### Device Login Issues

- **Device code not working**:
  - Ensure the device code hasn't expired (codes expire after a few minutes)
  - Verify you're entering the code on the correct authorization page
  - Try starting a new device login flow

- **Auto-polling not working**:
  - The app automatically checks for authorization every few seconds
  - You can also manually click "Check status" if needed

