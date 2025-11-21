# Desktop Client Documentation

This section covers the architecture, development, and deployment of the cross-platform desktop VPN clients built with Rust and Tauri.

---

## Client Overview

The VPN desktop clients provide a native, privacy-focused experience across Windows, macOS, and Linux platforms. The architecture emphasizes:

- **Performance**: Rust networking core for minimal latency and high throughput
- **Security**: Local-first operation with kill switch and DNS leak protection  
- **Privacy**: No telemetry collection, local credential storage
- **User Experience**: Native OS integration with system tray and notifications
- **Reliability**: Automatic reconnection and failover capabilities

---

## Architecture

### Component Structure

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Frontend                           │
│  ┌─────────────────┐    ┌─────────────────┐                 │
│  │   HTML/CSS/JS   │◄──►│  Tauri Commands │                 │
│  │   (React/Vue)   │    │   (Rust APIs)   │                 │
│  └─────────────────┘    └─────────────────┘                 │
├─────────────────────────────────────────────────────────────┤
│                     VPN Core (Rust)                         │
│  ┌─────────────────┐    ┌─────────────────┐                 │
│  │ Connection Mgmt │    │ Profile Storage │                 │
│  │ • Auth Tokens   │    │ • User Settings │                 │
│  │ • Session State │    │ • Server Lists  │                 │
│  └─────────────────┘    └─────────────────┘                 │
│  ┌─────────────────┐    ┌─────────────────┐                 │
│  │  WireGuard      │    │  Network Utils  │                 │
│  │ • Key Exchange  │    │ • DNS Config    │                 │
│  │ • Tunnel Setup  │    │ • Route Tables  │                 │
│  └─────────────────┘    └─────────────────┘                 │
├─────────────────────────────────────────────────────────────┤
│                  Platform Layer (Rust)                      │
│  ┌─────────────────┐    ┌──────────────────┐                │
│  │  TUN Interface  │    │   Kill Switch    │                │
│  │ • Packet Capture│    │ • Firewall Rules │                │
│  │ • Traffic Route │    │ • Leak Prevention│                │
│  └─────────────────┘    └──────────────────┘                │
│  ┌──────────────────┐    ┌─────────────────┐                │
│  │  DNS Management  │    │  System Tray    │                │
│  │ • Resolver Config│    │ • Notifications │                │
│  │ • Leak Detection │    │ • Status Updates│                │
│  └──────────────────┘    └─────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

### Technology Stack

**Frontend**:
- **Framework**: Tauri (Rust + Web technologies)
- **UI Library**: React with TypeScript
- **Styling**: Tailwind CSS for responsive design
- **State Management**: Zustand for client state
- **Build Tool**: Vite for fast development and building

**Backend (Rust)**:
- **Networking**: `tokio` for async I/O
- **WireGuard**: `wireguard-rs` for userspace implementation
- **HTTP Client**: `reqwest` for API communication
- **Serialization**: `serde` for JSON/config handling
- **Cryptography**: `ring` for key management
- **Platform APIs**: OS-specific crates for system integration

---

## Platform-Specific Implementation

### Windows Implementation

#### TUN Interface Management

```rust
// client/platform-windows/src/tun_backend.rs
use windows::Win32::NetworkManagement::IpHelper::*;
use windows::Win32::NetworkManagement::Ndis::*;

pub struct WindowsTunBackend {
    adapter_guid: String,
    interface_index: u32,
}

impl WindowsTunBackend {
    pub fn new() -> Result<Self, TunError> {
        // Create WinTUN adapter
        let adapter = unsafe {
            WintunCreateAdapter(
                w!("VPN Client"),
                w!("VPN Tunnel"),
                std::ptr::null(),
            )?
        };
        
        Ok(Self {
            adapter_guid: get_adapter_guid(&adapter)?,
            interface_index: get_interface_index(&adapter)?,
        })
    }
    
    pub fn configure_interface(&self, config: &TunConfig) -> Result<(), TunError> {
        // Set IP address and netmask
        unsafe {
            SetAdapterIpAddress(
                self.interface_index,
                config.ip_address.octets().as_ptr(),
                config.netmask.octets().as_ptr(),
            )?;
        }
        
        // Configure routes
        for route in &config.routes {
            self.add_route(route)?;
        }
        
        Ok(())
    }
    
    fn add_route(&self, route: &Route) -> Result<(), TunError> {
        let mut row = MIB_IPFORWARDROW::default();
        row.dwForwardDest = u32::from_be_bytes(route.destination.octets());
        row.dwForwardMask = u32::from_be_bytes(route.netmask.octets());
        row.dwForwardNextHop = u32::from_be_bytes(route.gateway.octets());
        row.dwForwardIfIndex = self.interface_index;
        row.dwForwardMetric1 = route.metric;
        
        unsafe {
            CreateIpForwardEntry(&row)?;
        }
        
        Ok(())
    }
}
```

#### Firewall Integration

```rust
// client/platform-windows/src/firewall.rs
use windows::Win32::NetworkManagement::WindowsFirewall::*;

pub struct WindowsFirewall {
    policy: INetFwPolicy2,
}

impl WindowsFirewall {
    pub fn new() -> Result<Self, FirewallError> {
        let policy: INetFwPolicy2 = unsafe {
            CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER)?
        };
        
        Ok(Self { policy })
    }
    
    pub fn enable_kill_switch(&self, vpn_interface: &str) -> Result<(), FirewallError> {
        // Block all traffic except through VPN interface
        let rule = self.create_block_rule("VPN Kill Switch - Block All")?;
        
        unsafe {
            rule.SetEnabled(VARIANT_TRUE)?;
            rule.SetDirection(NET_FW_RULE_DIR_OUT)?;
            rule.SetAction(NET_FW_ACTION_BLOCK)?;
            rule.SetInterfaceTypes(&BSTR::from("All"))?;
            
            self.policy.Rules()?.Add(&rule)?;
        }
        
        // Allow traffic through VPN interface
        let allow_rule = self.create_allow_rule("VPN Kill Switch - Allow VPN")?;
        
        unsafe {
            allow_rule.SetEnabled(VARIANT_TRUE)?;
            allow_rule.SetDirection(NET_FW_RULE_DIR_OUT)?;
            allow_rule.SetAction(NET_FW_ACTION_ALLOW)?;
            allow_rule.SetInterfaceTypes(&BSTR::from(vpn_interface))?;
            
            self.policy.Rules()?.Add(&allow_rule)?;
        }
        
        Ok(())
    }
}
```

### macOS Implementation

#### TUN Interface Management

```rust
// client/platform-macos/src/tun_backend.rs
use core_foundation::base::*;
use system_configuration::network_configuration::*;

pub struct MacOSTunBackend {
    interface_name: String,
    fd: i32,
}

impl MacOSTunBackend {
    pub fn new() -> Result<Self, TunError> {
        // Create TUN interface using utun
        let fd = unsafe {
            let fd = libc::socket(libc::PF_SYSTEM, libc::SOCK_DGRAM, libc::SYSPROTO_CONTROL);
            if fd < 0 {
                return Err(TunError::CreateFailed);
            }
            
            // Connect to utun control
            let mut info = libc::ctl_info {
                ctl_id: 0,
                ctl_name: [0; 96],
            };
            
            let name = b"com.apple.net.utun_control\0";
            libc::memcpy(
                info.ctl_name.as_mut_ptr() as *mut _,
                name.as_ptr() as *const _,
                name.len(),
            );
            
            if libc::ioctl(fd, libc::CTLIOCGINFO, &mut info) < 0 {
                libc::close(fd);
                return Err(TunError::CreateFailed);
            }
            
            // Connect to utun
            let mut addr = libc::sockaddr_ctl {
                sc_len: std::mem::size_of::<libc::sockaddr_ctl>() as u8,
                sc_family: libc::AF_SYSTEM as u8,
                ss_sysaddr: libc::AF_SYS_CONTROL as u16,
                sc_id: info.ctl_id,
                sc_unit: 0, // Let system assign unit number
                sc_reserved: [0; 5],
            };
            
            if libc::connect(
                fd,
                &addr as *const _ as *const libc::sockaddr,
                std::mem::size_of::<libc::sockaddr_ctl>() as u32,
            ) < 0 {
                libc::close(fd);
                return Err(TunError::CreateFailed);
            }
            
            fd
        };
        
        let interface_name = format!("utun{}", self.get_unit_number(fd)?);
        
        Ok(Self {
            interface_name,
            fd,
        })
    }
    
    pub fn configure_interface(&self, config: &TunConfig) -> Result<(), TunError> {
        // Configure IP address using ifconfig
        let output = std::process::Command::new("ifconfig")
            .args(&[
                &self.interface_name,
                &config.ip_address.to_string(),
                &config.peer_address.to_string(),
                "up"
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(TunError::ConfigFailed);
        }
        
        // Add routes
        for route in &config.routes {
            self.add_route(route)?;
        }
        
        Ok(())
    }
    
    fn add_route(&self, route: &Route) -> Result<(), TunError> {
        let output = std::process::Command::new("route")
            .args(&[
                "add",
                "-net",
                &format!("{}/{}", route.destination, route.prefix_len),
                "-interface",
                &self.interface_name,
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(TunError::RouteFailed);
        }
        
        Ok(())
    }
}
```

#### DNS Management

```rust
// client/platform-macos/src/dns.rs
use system_configuration::dynamic_store::*;

pub struct MacOSDNSManager {
    store: SCDynamicStore,
}

impl MacOSDNSManager {
    pub fn new() -> Result<Self, DNSError> {
        let store = SCDynamicStoreBuilder::new("VPN Client").build();
        Ok(Self { store })
    }
    
    pub fn set_dns_servers(&self, servers: &[IpAddr]) -> Result<(), DNSError> {
        let dns_dict = CFDictionary::from_CFType_pairs(&[
            (
                CFString::new("ServerAddresses"),
                CFArray::from_copyable(&servers.iter()
                    .map(|ip| CFString::new(&ip.to_string()))
                    .collect::<Vec<_>>())
            ),
        ]);
        
        let key = CFString::new("State:/Network/Service/VPN/DNS");
        self.store.set(&key, dns_dict)?;
        
        Ok(())
    }
    
    pub fn restore_dns(&self) -> Result<(), DNSError> {
        let key = CFString::new("State:/Network/Service/VPN/DNS");
        self.store.remove(&key)?;
        Ok(())
    }
}
```

### Linux Implementation

#### TUN Interface Management

```rust
// client/platform-linux/src/tun_backend.rs
use nix::sys::socket::*;
use nix::unistd::*;

pub struct LinuxTunBackend {
    interface_name: String,
    fd: i32,
}

impl LinuxTunBackend {
    pub fn new() -> Result<Self, TunError> {
        // Open TUN device
        let fd = unsafe {
            libc::open(
                b"/dev/net/tun\0".as_ptr() as *const i8,
                libc::O_RDWR | libc::O_CLOEXEC,
            )
        };
        
        if fd < 0 {
            return Err(TunError::CreateFailed);
        }
        
        // Configure TUN interface
        let mut ifr: libc::ifreq = unsafe { std::mem::zeroed() };
        ifr.ifr_ifru.ifru_flags = (libc::IFF_TUN | libc::IFF_NO_PI) as i16;
        
        // Let kernel assign interface name
        let interface_name = unsafe {
            if libc::ioctl(fd, libc::TUNSETIFF, &mut ifr) < 0 {
                libc::close(fd);
                return Err(TunError::CreateFailed);
            }
            
            std::ffi::CStr::from_ptr(ifr.ifr_name.as_ptr())
                .to_string_lossy()
                .to_string()
        };
        
        Ok(Self {
            interface_name,
            fd,
        })
    }
    
    pub fn configure_interface(&self, config: &TunConfig) -> Result<(), TunError> {
        // Set interface up and configure IP
        let output = std::process::Command::new("ip")
            .args(&[
                "addr", "add",
                &format!("{}/{}", config.ip_address, config.prefix_len),
                "dev", &self.interface_name,
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(TunError::ConfigFailed);
        }
        
        // Bring interface up
        let output = std::process::Command::new("ip")
            .args(&["link", "set", "dev", &self.interface_name, "up"])
            .output()?;
        
        if !output.status.success() {
            return Err(TunError::ConfigFailed);
        }
        
        // Add routes
        for route in &config.routes {
            self.add_route(route)?;
        }
        
        Ok(())
    }
    
    fn add_route(&self, route: &Route) -> Result<(), TunError> {
        let output = std::process::Command::new("ip")
            .args(&[
                "route", "add",
                &format!("{}/{}", route.destination, route.prefix_len),
                "dev", &self.interface_name,
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(TunError::RouteFailed);
        }
        
        Ok(())
    }
}
```

---

## Core VPN Functionality

### Connection Management

```rust
// client/vpn-core/src/lib.rs
use tokio::sync::{mpsc, RwLock};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected { server: ServerInfo, since: Instant },
    Reconnecting { attempt: u32 },
    Disconnecting,
}

pub struct VPNManager {
    state: Arc<RwLock<ConnectionState>>,
    config: VPNConfig,
    platform: Box<dyn PlatformInterface>,
    event_tx: mpsc::UnboundedSender<VPNEvent>,
}

impl VPNManager {
    pub fn new(config: VPNConfig) -> Result<Self, VPNError> {
        let platform = create_platform_interface()?;
        let (event_tx, _) = mpsc::unbounded_channel();
        
        Ok(Self {
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            config,
            platform,
            event_tx,
        })
    }
    
    pub async fn connect(&self, server: ServerInfo) -> Result<(), VPNError> {
        let mut state = self.state.write().await;
        *state = ConnectionState::Connecting;
        drop(state);
        
        self.send_event(VPNEvent::StateChanged(ConnectionState::Connecting));
        
        // Generate WireGuard keypair
        let keypair = WireGuardKeypair::generate();
        
        // Request VPN configuration from server
        let vpn_config = self.request_vpn_config(&server, &keypair).await?;
        
        // Configure platform-specific networking
        self.platform.configure_interface(&vpn_config.interface).await?;
        self.platform.enable_kill_switch(&vpn_config.interface.name).await?;
        
        // Establish WireGuard tunnel
        let tunnel = WireGuardTunnel::new(keypair, vpn_config.peer)?;
        tunnel.start().await?;
        
        // Update state
        let mut state = self.state.write().await;
        *state = ConnectionState::Connected {
            server: server.clone(),
            since: Instant::now(),
        };
        
        self.send_event(VPNEvent::Connected(server));
        
        // Start connection monitoring
        self.start_monitoring(tunnel).await;
        
        Ok(())
    }
    
    pub async fn disconnect(&self) -> Result<(), VPNError> {
        let mut state = self.state.write().await;
        *state = ConnectionState::Disconnecting;
        drop(state);
        
        self.send_event(VPNEvent::StateChanged(ConnectionState::Disconnecting));
        
        // Disable kill switch
        self.platform.disable_kill_switch().await?;
        
        // Tear down tunnel
        self.platform.teardown_interface().await?;
        
        // Update state
        let mut state = self.state.write().await;
        *state = ConnectionState::Disconnected;
        
        self.send_event(VPNEvent::Disconnected);
        
        Ok(())
    }
    
    async fn start_monitoring(&self, tunnel: WireGuardTunnel) {
        let state = Arc::clone(&self.state);
        let event_tx = self.event_tx.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Check tunnel health
                match tunnel.get_stats().await {
                    Ok(stats) => {
                        if stats.last_handshake.elapsed() > Duration::from_secs(180) {
                            // Tunnel appears dead, attempt reconnection
                            event_tx.send(VPNEvent::TunnelError("Handshake timeout".into())).ok();
                        }
                    }
                    Err(e) => {
                        event_tx.send(VPNEvent::TunnelError(e.to_string())).ok();
                    }
                }
            }
        });
    }
}
```

### Authentication Integration

```rust
// client/vpn-core/src/netauth.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AuthClient {
    client: Client,
    base_url: String,
    tokens: Arc<RwLock<Option<TokenPair>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Instant,
}

impl AuthClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
            base_url,
            tokens: Arc::new(RwLock::new(None)),
        }
    }
    
    pub async fn login(&self, email: &str, password: &str) -> Result<(), AuthError> {
        let response = self.client
            .post(&format!("{}/auth/login", self.base_url))
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(AuthError::InvalidCredentials);
        }
        
        let token_response: TokenResponse = response.json().await?;
        
        let tokens = TokenPair {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: Instant::now() + Duration::from_secs(token_response.expires_in),
        };
        
        *self.tokens.write().await = Some(tokens);
        
        Ok(())
    }
    
    pub async fn get_valid_token(&self) -> Result<String, AuthError> {
        let tokens = self.tokens.read().await;
        
        match &*tokens {
            Some(tokens) if tokens.expires_at > Instant::now() + Duration::from_secs(60) => {
                Ok(tokens.access_token.clone())
            }
            Some(tokens) => {
                drop(tokens);
                self.refresh_token().await?;
                let tokens = self.tokens.read().await;
                Ok(tokens.as_ref().unwrap().access_token.clone())
            }
            None => Err(AuthError::NotAuthenticated),
        }
    }
    
    async fn refresh_token(&self) -> Result<(), AuthError> {
        let refresh_token = {
            let tokens = self.tokens.read().await;
            tokens.as_ref()
                .ok_or(AuthError::NotAuthenticated)?
                .refresh_token.clone()
        };
        
        let response = self.client
            .post(&format!("{}/auth/refresh", self.base_url))
            .json(&serde_json::json!({
                "refresh_token": refresh_token
            }))
            .send()
            .await?;
        
        if !response.status().is_success() {
            *self.tokens.write().await = None;
            return Err(AuthError::RefreshFailed);
        }
        
        let token_response: TokenResponse = response.json().await?;
        
        let new_tokens = TokenPair {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: Instant::now() + Duration::from_secs(token_response.expires_in),
        };
        
        *self.tokens.write().await = Some(new_tokens);
        
        Ok(())
    }
}
```

---

## User Interface

### Tauri Integration

#### Main Application Window

```rust
// client/vpn-gui/src/main.rs
use tauri::{CustomMenuItem, Menu, MenuItem, Submenu, SystemTray, SystemTrayMenu};

#[tauri::command]
async fn connect_vpn(server_id: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let vpn_manager = state.vpn_manager.lock().await;
    
    // Get server info
    let server = state.server_list.get(&server_id)
        .ok_or("Server not found")?;
    
    // Connect to VPN
    vpn_manager.connect(server.clone()).await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
async fn disconnect_vpn(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let vpn_manager = state.vpn_manager.lock().await;
    vpn_manager.disconnect().await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
async fn get_connection_status(state: tauri::State<'_, AppState>) -> Result<ConnectionStatus, String> {
    let vpn_manager = state.vpn_manager.lock().await;
    let status = vpn_manager.get_status().await;
    
    Ok(ConnectionStatus {
        state: status.state,
        server: status.server,
        connected_since: status.connected_since,
        bytes_sent: status.bytes_sent,
        bytes_received: status.bytes_received,
    })
}

fn main() {
    let context = tauri::generate_context!();
    
    // Create system tray
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("show", "Show VPN Client"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("connect", "Quick Connect"))
        .add_item(CustomMenuItem::new("disconnect", "Disconnect"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("quit", "Quit"));
    
    let system_tray = SystemTray::new().with_menu(tray_menu);
    
    tauri::Builder::default()
        .menu(create_menu())
        .system_tray(system_tray)
        .on_system_tray_event(handle_system_tray_event)
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            connect_vpn,
            disconnect_vpn,
            get_connection_status,
            get_server_list,
            login_user,
            logout_user
        ])
        .setup(|app| {
            // Initialize VPN manager
            let app_state = app.state::<AppState>();
            tauri::async_runtime::spawn(async move {
                app_state.initialize().await;
            });
            
            Ok(())
        })
        .run(context)
        .expect("error while running tauri application");
}
```

#### Frontend React Components

```typescript
// client/vpn-gui/ui/src/components/ConnectionPanel.tsx
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';

interface ConnectionStatus {
  state: 'disconnected' | 'connecting' | 'connected' | 'disconnecting';
  server?: ServerInfo;
  connected_since?: string;
  bytes_sent: number;
  bytes_received: number;
}

export const ConnectionPanel: React.FC = () => {
  const [status, setStatus] = useState<ConnectionStatus | null>(null);
  const [selectedServer, setSelectedServer] = useState<string>('');
  const [isConnecting, setIsConnecting] = useState(false);

  useEffect(() => {
    // Listen for VPN events
    const unlisten = listen('vpn-event', (event) => {
      console.log('VPN Event:', event.payload);
      refreshStatus();
    });

    // Initial status fetch
    refreshStatus();

    return () => {
      unlisten.then(f => f());
    };
  }, []);

  const refreshStatus = async () => {
    try {
      const status = await invoke<ConnectionStatus>('get_connection_status');
      setStatus(status);
    } catch (error) {
      console.error('Failed to get status:', error);
    }
  };

  const handleConnect = async () => {
    if (!selectedServer) return;
    
    setIsConnecting(true);
    try {
      await invoke('connect_vpn', { serverId: selectedServer });
    } catch (error) {
      console.error('Connection failed:', error);
      // Show error notification
    } finally {
      setIsConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    try {
      await invoke('disconnect_vpn');
    } catch (error) {
      console.error('Disconnection failed:', error);
    }
  };

  const formatBytes = (bytes: number) => {
    const units = ['B', 'KB', 'MB', 'GB'];
    let size = bytes;
    let unitIndex = 0;
    
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex++;
    }
    
    return `${size.toFixed(1)} ${units[unitIndex]}`;
  };

  return (
    <div className="connection-panel p-6 bg-white rounded-lg shadow-lg">
      <div className="status-section mb-6">
        <div className={`status-indicator ${status?.state || 'disconnected'}`}>
          <div className="status-dot"></div>
          <span className="status-text">
            {status?.state === 'connected' ? 'Connected' :
             status?.state === 'connecting' ? 'Connecting...' :
             status?.state === 'disconnecting' ? 'Disconnecting...' :
             'Disconnected'}
          </span>
        </div>
        
        {status?.server && (
          <div className="server-info mt-2">
            <span className="server-name">{status.server.name}</span>
            <span className="server-location">{status.server.location}</span>
          </div>
        )}
      </div>

      {status?.state === 'connected' && (
        <div className="stats-section mb-6">
          <div className="stat-item">
            <span className="stat-label">Uploaded:</span>
            <span className="stat-value">{formatBytes(status.bytes_sent)}</span>
          </div>
          <div className="stat-item">
            <span className="stat-label">Downloaded:</span>
            <span className="stat-value">{formatBytes(status.bytes_received)}</span>
          </div>
          {status.connected_since && (
            <div className="stat-item">
              <span className="stat-label">Connected since:</span>
              <span className="stat-value">
                {new Date(status.connected_since).toLocaleTimeString()}
              </span>
            </div>
          )}
        </div>
      )}

      <div className="controls-section">
        {status?.state === 'disconnected' ? (
          <div>
            <ServerSelector 
              value={selectedServer}
              onChange={setSelectedServer}
              className="mb-4"
            />
            <button
              onClick={handleConnect}
              disabled={!selectedServer || isConnecting}
              className="connect-button w-full py-3 px-6 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50"
            >
              {isConnecting ? 'Connecting...' : 'Connect'}
            </button>
          </div>
        ) : (
          <button
            onClick={handleDisconnect}
            disabled={status?.state === 'disconnecting'}
            className="disconnect-button w-full py-3 px-6 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50"
          >
            {status?.state === 'disconnecting' ? 'Disconnecting...' : 'Disconnect'}
          </button>
        )}
      </div>
    </div>
  );
};
```

---

## Build and Distribution

### Cross-Platform Building

#### Build Configuration

```toml
# client/vpn-gui/Cargo.toml
[package]
name = "vpn-gui"
version = "1.0.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "1.0", features = [] }

[dependencies]
tauri = { version = "1.0", features = ["api-all", "system-tray", "updater"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
vpn-core = { path = "../vpn-core" }

[target.'cfg(windows)'.dependencies]
platform-windows = { path = "../platform-windows" }

[target.'cfg(target_os = "macos")'.dependencies]
platform-macos = { path = "../platform-macos" }

[target.'cfg(target_os = "linux")'.dependencies]
platform-linux = { path = "../platform-linux" }

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
```

#### Tauri Configuration

```json
// client/vpn-gui/ui/src-tauri/tauri.conf.json
{
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devPath": "http://localhost:1420",
    "distDir": "../dist",
    "withGlobalTauri": false
  },
  "package": {
    "productName": "VPN Client",
    "version": "1.0.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "all": false,
        "open": true
      },
      "notification": {
        "all": true
      },
      "systemTray": {
        "all": true
      }
    },
    "bundle": {
      "active": true,
      "targets": "all",
      "identifier": "com.vpn.client",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "resources": [],
      "externalBin": [],
      "copyright": "",
      "category": "Utility",
      "shortDescription": "Privacy-focused VPN client",
      "longDescription": "A fast, secure, and privacy-focused VPN client built with Rust and Tauri.",
      "windows": {
        "certificateThumbprint": null,
        "digestAlgorithm": "sha256",
        "timestampUrl": ""
      },
      "macOS": {
        "frameworks": [],
        "minimumSystemVersion": "10.13",
        "exceptionDomain": "",
        "signingIdentity": null,
        "providerShortName": null,
        "entitlements": null
      },
      "linux": {
        "deb": {
          "depends": ["libwebkit2gtk-4.0-37", "libgtk-3-0"]
        }
      }
    },
    "security": {
      "csp": null
    },
    "updater": {
      "active": true,
      "endpoints": [
        "https://updates.vpn.example.com/{{target}}/{{arch}}/{{current_version}}"
      ],
      "dialog": true,
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEFBQUFBQUFBQUFBQUFBQUE="
    },
    "windows": [
      {
        "fullscreen": false,
        "resizable": true,
        "title": "VPN Client",
        "width": 800,
        "height": 600,
        "minWidth": 600,
        "minHeight": 400
      }
    ],
    "systemTray": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true,
      "menuOnLeftClick": false
    }
  }
}
```

### Automated Building

#### GitHub Actions Workflow

```yaml
# .github/workflows/build-clients.yml
name: Build Desktop Clients

on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        platform: [macos-latest, ubuntu-20.04, windows-latest]

    runs-on: ${{ matrix.platform }}
    steps:
      - name: Checkout repository
        uses: actions/checkout@v3

      - name: Install dependencies (ubuntu only)
        if: matrix.platform == 'ubuntu-20.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Rust setup
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: './client -> target'

      - name: Node.js setup
        uses: actions/setup-node@v3
        with:
          node-version: 18
          cache: 'npm'
          cache-dependency-path: client/vpn-gui/ui/package-lock.json

      - name: Install frontend dependencies
        run: |
          cd client/vpn-gui/ui
          npm ci

      - name: Build the app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_PRIVATE_KEY: ${{ secrets.TAURI_PRIVATE_KEY }}
          TAURI_KEY_PASSWORD: ${{ secrets.TAURI_KEY_PASSWORD }}
        with:
          projectPath: client/vpn-gui
          tagName: ${{ github.ref_name }}
          releaseName: 'VPN Client v__VERSION__'
          releaseBody: 'See the assets to download this version and install.'
          releaseDraft: true
          prerelease: false
          includeDebug: false
```

### Installation Packages

#### Windows Installer (WiX)

```xml
<!-- client/installers/windows/installer.wxs -->
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="*" Name="VPN Client" Language="1033" Version="1.0.0" 
           Manufacturer="VPN Company" UpgradeCode="12345678-1234-1234-1234-123456789012">
    
    <Package InstallerVersion="200" Compressed="yes" InstallScope="perMachine" />
    
    <MajorUpgrade DowngradeErrorMessage="A newer version is already installed." />
    
    <MediaTemplate EmbedCab="yes" />
    
    <Feature Id="ProductFeature" Title="VPN Client" Level="1">
      <ComponentGroupRef Id="ProductComponents" />
    </Feature>
    
    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFilesFolder">
        <Directory Id="INSTALLFOLDER" Name="VPN Client" />
      </Directory>
      <Directory Id="ProgramMenuFolder">
        <Directory Id="ApplicationProgramsFolder" Name="VPN Client"/>
      </Directory>
    </Directory>
    
    <ComponentGroup Id="ProductComponents" Directory="INSTALLFOLDER">
      <Component Id="MainExecutable" Guid="*">
        <File Id="VPNClientExe" Source="$(var.SourceDir)\vpn-client.exe" KeyPath="yes">
          <Shortcut Id="ApplicationStartMenuShortcut" Directory="ApplicationProgramsFolder" 
                    Name="VPN Client" WorkingDirectory="INSTALLFOLDER" Icon="VPNClient.exe" IconIndex="0" Advertise="yes" />
        </File>
      </Component>
      
      <Component Id="TAPDriver" Guid="*">
        <File Id="TAPDriverInf" Source="$(var.SourceDir)\tap\OemVista.inf" KeyPath="yes" />
        <File Id="TAPDriverSys" Source="$(var.SourceDir)\tap\tap0901.sys" />
        <File Id="TAPDriverCat" Source="$(var.SourceDir)\tap\tap0901.cat" />
      </Component>
    </ComponentGroup>
    
    <CustomAction Id="InstallTAPDriver" Directory="INSTALLFOLDER" 
                  ExeCommand="[SystemFolder]PnPUtil.exe /add-driver tap\OemVista.inf /install" 
                  Execute="deferred" Impersonate="no" />
    
    <InstallExecuteSequence>
      <Custom Action="InstallTAPDriver" After="InstallFiles">NOT Installed</Custom>
    </InstallExecuteSequence>
    
  </Product>
</Wix>
```

#### macOS Package

```bash
#!/bin/bash
# client/installers/macos/build_pkg.sh

set -euo pipefail

APP_NAME="VPN Client"
APP_BUNDLE="VPN Client.app"
PKG_NAME="VPN-Client-Installer.pkg"
IDENTIFIER="com.vpn.client"
VERSION="1.0.0"

# Create temporary directory structure
TEMP_DIR=$(mktemp -d)
PAYLOAD_DIR="$TEMP_DIR/payload"
SCRIPTS_DIR="$TEMP_DIR/scripts"

mkdir -p "$PAYLOAD_DIR/Applications"
mkdir -p "$SCRIPTS_DIR"

# Copy application bundle
cp -R "target/release/bundle/macos/$APP_BUNDLE" "$PAYLOAD_DIR/Applications/"

# Create preinstall script
cat > "$SCRIPTS_DIR/preinstall" << 'EOF'
#!/bin/bash
# Stop existing VPN client if running
pkill -f "VPN Client" || true

# Remove old installation
rm -rf "/Applications/VPN Client.app"

exit 0
EOF

# Create postinstall script  
cat > "$SCRIPTS_DIR/postinstall" << 'EOF'
#!/bin/bash
# Set proper permissions
chown -R root:admin "/Applications/VPN Client.app"
chmod -R 755 "/Applications/VPN Client.app"

# Install system extension if needed
# (This would require additional entitlements and notarization)

exit 0
EOF

chmod +x "$SCRIPTS_DIR/preinstall"
chmod +x "$SCRIPTS_DIR/postinstall"

# Build package
pkgbuild \
    --root "$PAYLOAD_DIR" \
    --scripts "$SCRIPTS_DIR" \
    --identifier "$IDENTIFIER" \
    --version "$VERSION" \
    --install-location "/" \
    "$PKG_NAME"

# Clean up
rm -rf "$TEMP_DIR"

echo "✅ Package created: $PKG_NAME"
```

#### Linux Packages

```bash
#!/bin/bash
# client/installers/linux/build_deb.sh

set -euo pipefail

PACKAGE_NAME="vpn-client"
VERSION="1.0.0"
ARCH="amd64"

# Create package directory structure
PKG_DIR="target/debian"
mkdir -p "$PKG_DIR/DEBIAN"
mkdir -p "$PKG_DIR/usr/bin"
mkdir -p "$PKG_DIR/usr/share/applications"
mkdir -p "$PKG_DIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$PKG_DIR/etc/systemd/system"

# Copy binary
cp "target/release/vpn-client" "$PKG_DIR/usr/bin/"

# Create desktop file
cat > "$PKG_DIR/usr/share/applications/vpn-client.desktop" << EOF
[Desktop Entry]
Name=VPN Client
Comment=Privacy-focused VPN client
Exec=/usr/bin/vpn-client
Icon=vpn-client
Terminal=false
Type=Application
Categories=Network;Security;
StartupNotify=true
EOF

# Copy icon
cp "client/icons/256x256.png" "$PKG_DIR/usr/share/icons/hicolor/256x256/apps/vpn-client.png"

# Create control file
cat > "$PKG_DIR/DEBIAN/control" << EOF
Package: $PACKAGE_NAME
Version: $VERSION
Section: net
Priority: optional
Architecture: $ARCH
Depends: libgtk-3-0, libwebkit2gtk-4.0-37
Maintainer: VPN Team <support@vpn.example.com>
Description: Privacy-focused VPN client
 A fast, secure, and privacy-focused VPN client built with Rust and Tauri.
 Provides strong encryption, kill switch protection, and DNS leak prevention.
EOF

# Create postinst script
cat > "$PKG_DIR/DEBIAN/postinst" << 'EOF'
#!/bin/bash
set -e

# Update desktop database
if command -v update-desktop-database > /dev/null; then
    update-desktop-database /usr/share/applications
fi

# Update icon cache
if command -v gtk-update-icon-cache > /dev/null; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor
fi

exit 0
EOF

chmod +x "$PKG_DIR/DEBIAN/postinst"

# Build package
dpkg-deb --build "$PKG_DIR" "${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"

echo "✅ Debian package created: ${PACKAGE_NAME}_${VERSION}_${ARCH}.deb"
```

---

## Testing and Quality Assurance

### Unit Testing

```rust
// client/vpn-core/tests/connection_test.rs
use vpn_core::*;
use tokio_test;

#[tokio::test]
async fn test_connection_lifecycle() {
    let config = VPNConfig::default();
    let manager = VPNManager::new(config).unwrap();
    
    // Test initial state
    let status = manager.get_status().await;
    assert_eq!(status.state, ConnectionState::Disconnected);
    
    // Test connection
    let server = ServerInfo {
        id: "test-server".to_string(),
        name: "Test Server".to_string(),
        location: "Test Location".to_string(),
        endpoint: "test.example.com:51820".to_string(),
    };
    
    manager.connect(server.clone()).await.unwrap();
    
    // Verify connected state
    let status = manager.get_status().await;
    assert_eq!(status.state, ConnectionState::Connected { server: server.clone(), since: _ });
    
    // Test disconnection
    manager.disconnect().await.unwrap();
    
    let status = manager.get_status().await;
    assert_eq!(status.state, ConnectionState::Disconnected);
}
```

### Integration Testing

```typescript
// client/vpn-gui/ui/tests/integration/connection.test.ts
import { test, expect } from '@playwright/test';

test.describe('VPN Connection Flow', () => {
  test('should connect and disconnect successfully', async ({ page }) => {
    await page.goto('/');
    
    // Wait for app to load
    await expect(page.locator('[data-testid="connection-panel"]')).toBeVisible();
    
    // Verify initial disconnected state
    await expect(page.locator('[data-testid="status-indicator"]')).toHaveClass(/disconnected/);
    
    // Select a server
    await page.selectOption('[data-testid="server-selector"]', 'test-server-1');
    
    // Click connect button
    await page.click('[data-testid="connect-button"]');
    
    // Wait for connection to establish
    await expect(page.locator('[data-testid="status-indicator"]')).toHaveClass(/connected/, { timeout: 10000 });
    
    // Verify connection stats are displayed
    await expect(page.locator('[data-testid="connection-stats"]')).toBeVisible();
    
    // Disconnect
    await page.click('[data-testid="disconnect-button"]');
    
    // Verify disconnected state
    await expect(page.locator('[data-testid="status-indicator"]')).toHaveClass(/disconnected/);
  });
  
  test('should handle connection errors gracefully', async ({ page }) => {
    await page.goto('/');
    
    // Mock network error
    await page.route('**/api/mesh/client-config', route => {
      route.fulfill({ status: 500, body: 'Server error' });
    });
    
    // Attempt connection
    await page.selectOption('[data-testid="server-selector"]', 'test-server-1');
    await page.click('[data-testid="connect-button"]');
    
    // Verify error notification
    await expect(page.locator('[data-testid="error-notification"]')).toBeVisible();
    await expect(page.locator('[data-testid="error-notification"]')).toContainText('Connection failed');
    
    // Verify still disconnected
    await expect(page.locator('[data-testid="status-indicator"]')).toHaveClass(/disconnected/);
  });
});
```