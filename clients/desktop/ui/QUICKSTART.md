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

## Testing with Local Node Container

1. **Start the node container**
   ```powershell
   # From repo root
   docker compose up -d node-agent
   docker compose logs --tail=50 node-agent
   ```
   - Copy the **server public key** from logs (e.g., `fcSezRGCA9+IMgoePomGrNG0XPbZ8qtB9iNwl3WKBFg=`)

2. **Generate client keys in the UI**
   - Open the Tauri app (via `pnpm tauri:dev`)
   - Use the UI to generate a keypair for your device
   - Copy the **client public key** from the UI

3. **Authorize your client key on the node**
   ```powershell
   # From repo root
   $env:WG_CLIENT_PUBKEY="<PASTE_CLIENT_PUBLIC_KEY>"
   docker compose up -d node-agent
   ```

4. **Configure connection in UI**
   - **Server public key**: `<SERVER_PUBLIC_KEY_FROM_LOGS>`
   - **Endpoint**: `127.0.0.1:51820` (localhost since container is on same machine)
   - **Client address**: `10.66.0.2/32`
   - **DNS**: `10.66.0.1`
   - **Allowed IPs**: Start with `10.66.0.0/24` for testing, then `0.0.0.0/0, ::/0` for full tunnel
   - **Persistent keepalive**: `25`

5. **Connect and verify**
   - Activate the connection in the UI
   - Test connectivity:
     ```powershell
     ping 10.66.0.1
     nslookup example.com 10.66.0.1
     curl -4 https://ifconfig.co
     ```

## Troubleshooting

- **"tauri is not recognized"**: Run `pnpm install` first to install `@tauri-apps/cli`
- **Build errors**: Ensure Visual Studio Build Tools with C++ workload is installed
- **Port 1420 in use**: Change port in `vite.config.ts` or kill the process using it
- **Connection issues**: Check `docker compose logs node-agent` for errors

