#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::Arc;

use anyhow::Result;
use base64::Engine as _;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::State;
use tokio::sync::Mutex;
use tokio::{sync::oneshot, task::JoinHandle};
use tauri::api::process::Command as TauriCommand;

#[derive(Debug)]
struct ServerConfig {
    base_auth_url: String,
    base_directory_url: String,
}

#[derive(Clone, Debug)]
struct AppState {
    server: Arc<Mutex<ServerConfig>>,
    token_store: Arc<Mutex<Option<TokenResponse>>>,
    bg: Arc<Mutex<BackgroundService>>,
}

impl AppState {
    fn new() -> Self {
        // Default dev URLs; can be overridden via env at runtime if needed
        let base_auth_url = std::env::var("VPN_AUTH_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
        let base_directory_url = std::env::var("VPN_DIRECTORY_URL").unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
        Self {
            server: Arc::new(Mutex::new(ServerConfig {
                base_auth_url,
                base_directory_url,
            })),
            token_store: Arc::new(Mutex::new(None)),
            bg: Arc::new(Mutex::new(BackgroundService::new())),
        }
    }
}

#[tauri::command]
async fn set_server_host(host: String, state: State<'_, AppState>) -> Result<(), String> {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err("host cannot be empty".to_string());
    }

    // Strip scheme if provided
    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed);

    // Take hostname/IP part before any port or path
    let host_part = without_scheme
        .split(&['/', ':'][..])
        .next()
        .ok_or_else(|| "invalid host".to_string())?;

    if host_part.is_empty() {
        return Err("invalid host".to_string());
    }

    let base_auth_url = format!("http://{}:8080", host_part);
    let base_directory_url = format!("http://{}:8081", host_part);

    let mut server = state.server.lock().await;
    server.base_auth_url = base_auth_url;
    server.base_directory_url = base_directory_url;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
    totp_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceLoginStartRequest {
    code_challenge_method: String,
    code_challenge: String,
    user_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceLoginStartResponse {
    device_code: String,
    expires_in: i64,
    interval: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceLoginPollRequest {
    device_code: String,
    code_verifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Region {
    id: String,
    country_code: String,
    city: String,
    status: String,
    latency_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeshPathRequest {
    region_id: String,
    mode: String, // "single" | "multi"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeRef {
    id: String,
    public_key: String,
    internal_wg_ip: Option<String>,
    egress_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeshPathResponse {
    mode: String,
    entry: NodeRef,
    #[serde(default)]
    exit: Option<NodeRef>,
}

#[tauri::command]
async fn login(req: LoginRequest, state: State<'_, AppState>) -> Result<TokenResponse, String> {
    let base_auth_url = {
        let server = state.server.lock().await;
        server.base_auth_url.clone()
    };
    let url = format!("{}/auth/login", base_auth_url);
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|_| "login failed".to_string()));
    }
    let tokens: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
    *state.token_store.lock().await = Some(tokens.clone());
    Ok(tokens)
}

fn gen_code_verifier() -> String {
    // RFC 7636 recommends 43-128 chars; use 64
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

fn s256_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

#[derive(Debug, Clone, Serialize)]
struct DeviceStartOut {
    device_code: String,
    expires_in: i64,
    interval: i64,
    code_verifier: String,
    code_challenge: String,
    code_challenge_method: String,
}

#[tauri::command]
async fn device_login_start(user_hint: Option<String>, state: State<'_, AppState>) -> Result<DeviceStartOut, String> {
    let verifier = gen_code_verifier();
    let challenge = s256_challenge(&verifier);
    let payload = DeviceLoginStartRequest {
        code_challenge_method: "S256".to_string(),
        code_challenge: challenge.clone(),
        user_hint,
    };
    let base_auth_url = {
        let server = state.server.lock().await;
        server.base_auth_url.clone()
    };
    let url = format!("{}/auth/device/start", base_auth_url);
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|_| "device start failed".to_string()));
    }
    let start: DeviceLoginStartResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(DeviceStartOut {
        device_code: start.device_code,
        expires_in: start.expires_in,
        interval: start.interval,
        code_verifier: verifier,
        code_challenge: challenge,
        code_challenge_method: "S256".to_string(),
    })
}

#[tauri::command]
async fn device_login_poll(device_code: String, code_verifier: String, state: State<'_, AppState>) -> Result<TokenResponse, String> {
    let base_auth_url = {
        let server = state.server.lock().await;
        server.base_auth_url.clone()
    };
    let url = format!("{}/auth/device/poll", base_auth_url);
    let client = reqwest::Client::new();
    let payload = DeviceLoginPollRequest {
        device_code,
        code_verifier,
    };
    let resp = client
        .post(url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|_| "authorization pending or failed".to_string()));
    }
    let tokens: TokenResponse = resp.json().await.map_err(|e| e.to_string())?;
    *state.token_store.lock().await = Some(tokens.clone());
    Ok(tokens)
}

#[tauri::command]
async fn fetch_regions(state: State<'_, AppState>) -> Result<Vec<Region>, String> {
    let base_directory_url = {
        let server = state.server.lock().await;
        server.base_directory_url.clone()
    };
    let url = format!("{}/regions", base_directory_url);
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|_| "failed to fetch regions".to_string()));
    }
    let list: Vec<Region> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(list)
}

#[tauri::command]
async fn app_status(state: State<'_, AppState>) -> Result<String, String> {
    let has_token = state.token_store.lock().await.is_some();
    let running = state.bg.lock().await.is_running();
    let mut s = if has_token { "authorized" } else { "unauthorized" }.to_string();
    if running {
        s.push_str(" • background:running");
    }
    Ok(s)
}

#[tauri::command]
async fn validate_mesh_selection(region_id: String, mode: String, state: State<'_, AppState>) -> Result<MeshPathResponse, String> {
    // Get access token from store
    let token_store = state.token_store.lock().await;
    let access_token = token_store.as_ref()
        .ok_or_else(|| "not authenticated".to_string())?
        .access_token.clone();
    drop(token_store);

    let base_directory_url = {
        let server = state.server.lock().await;
        server.base_directory_url.clone()
    };
    let url = format!("{}/mesh/path", base_directory_url);
    let client = reqwest::Client::new();
    let payload = MeshPathRequest { region_id, mode };
    let resp = client
        .post(url)
        .bearer_auth(&access_token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|_| "mesh selection failed".to_string()));
    }
    let out: MeshPathResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(out)
}

// Placeholder for future background service management using platform adapters and CLI-compatible loops.
// Not exposed yet as a command to avoid half-baked behavior.

#[derive(Debug)]
struct BackgroundService {
    handle: Option<JoinHandle<()>>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl BackgroundService {
    fn new() -> Self {
        Self { handle: None, stop_tx: None }
    }
    fn is_running(&self) -> bool {
        self.handle.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AutoConnectArgs {
    profile: String,
    iface: Option<String>,
    mtu: Option<u32>,
    trusted_ssids: Vec<String>,
    interval: u64,
}

#[tauri::command]
async fn start_autoconnect(args: AutoConnectArgs, state: State<'_, AppState>) -> Result<(), String> {
    use vpn_core::{FileProfileStore, RealCommandExecutor, VpnAdapter};
    let mut bg = state.bg.lock().await;
    if bg.is_running() {
        return Err("already running".to_string());
    }
    let (tx, rx) = oneshot::channel::<()>();
    #[cfg(target_os = "windows")]
    let adapter = {
        use platform_windows::WindowsAdapter;
        let exec = Arc::new(RealCommandExecutor);
        let creds = Arc::new(vpn_core::OsKeyringCredentialStore);
        let profiles = Arc::new(FileProfileStore::new("vpn_mvp").map_err(|e| e.to_string())?);
        WindowsAdapter::new(exec.clone(), creds, profiles.clone())
    };
    #[cfg(target_os = "macos")]
    let adapter = {
        use platform_macos::MacosAdapter;
        let exec = Arc::new(vpn_core::RealCommandExecutor);
        let creds = Arc::new(vpn_core::OsKeyringCredentialStore);
        let profiles = Arc::new(FileProfileStore::new("vpn_mvp").map_err(|e| e.to_string())?);
        MacosAdapter::new(exec.clone(), creds, profiles.clone())
    };
    #[cfg(target_os = "linux")]
    let adapter = {
        use platform_linux::LinuxNmAdapter;
        let exec = Arc::new(vpn_core::RealCommandExecutor);
        let profiles = Arc::new(FileProfileStore::new("vpn_mvp").map_err(|e| e.to_string())?);
        LinuxNmAdapter::new(exec.clone(), profiles.clone())
    };

    let exec = Arc::new(vpn_core::RealCommandExecutor);
    let name = args.profile.clone();
    let iface = args.iface.clone();
    let mtu = args.mtu;
    let ssids = args.trusted_ssids.clone();
    let interval = args.interval;
    let handle = tokio::spawn(async move {
        let mut rx = rx;
        let mut connection_logged = false;
        let mut last_stats_time = std::time::Instant::now();
        loop {
            let ssid = current_ssid(&*exec).await.unwrap_or_default();
            let trusted = !ssid.is_empty() && ssids.iter().any(|t| t == &ssid);
            let status = adapter.status(&name).await.unwrap_or(vpn_core::ConnectionState::Unknown);
            
            // Log connection status changes
            match status {
                vpn_core::ConnectionState::Connected => {
                    if !connection_logged {
                        println!("[VPN] ✓ Connected to VPN profile: {}", name);
                        eprintln!("[VPN] Connection established - Windows is now routing traffic through VPN");
                        connection_logged = true;
                    }
                    
                    // Log packet stats every 5 seconds
                    if last_stats_time.elapsed().as_secs() >= 5 {
                        let _ = log_vpn_packet_stats(&*exec, &name).await;
                        last_stats_time = std::time::Instant::now();
                    }
                },
                vpn_core::ConnectionState::Disconnected => {
                    if connection_logged {
                        println!("[VPN] ✗ Disconnected from VPN profile: {}", name);
                        eprintln!("[VPN] Connection lost - Windows is no longer routing traffic through VPN");
                        connection_logged = false;
                    }
                },
                _ => {}
            }
            
            if trusted {
                if matches!(status, vpn_core::ConnectionState::Connected | vpn_core::ConnectionState::Connecting) {
                    let _ = adapter.disconnect(&name).await;
                }
            } else {
                if !matches!(status, vpn_core::ConnectionState::Connected | vpn_core::ConnectionState::Connecting) {
                    let _ = adapter.connect(&name).await;
                    if let (Some(iface), Some(mtu)) = (&iface, mtu) {
                        let _ = set_mtu(&*exec, iface, mtu).await;
                    }
                }
            }
            let sleep = tokio::time::sleep(std::time::Duration::from_secs(interval));
            tokio::select! {
                _ = sleep => {},
                _ = &mut rx => { break; }
            }
        }
    });
    bg.handle = Some(handle);
    bg.stop_tx = Some(tx);
    Ok(())
}

#[tauri::command]
async fn stop_autoconnect(state: State<'_, AppState>) -> Result<(), String> {
    let mut bg = state.bg.lock().await;
    if let Some(tx) = bg.stop_tx.take() {
        let _ = tx.send(());
    }
    if let Some(handle) = bg.handle.take() {
        let _ = handle.abort();
    }
    Ok(())
}

async fn current_ssid(exec: &dyn vpn_core::CommandExecutor) -> Result<String, vpn_core::CoreError> {
    #[cfg(target_os = "windows")]
    {
        let out = exec.run("netsh", &["wlan", "show", "interfaces"]).await?;
        if out.status == 0 {
            for line in out.stdout.lines() {
                let l = line.trim();
                if l.starts_with("SSID") {
                    if let Some(idx) = l.find(':') {
                        return Ok(l[idx + 1..].trim().to_string());
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = exec.run("networksetup", &["-getairportnetwork", "Wi-Fi"]).await?;
        if out.status == 0 {
            if let Some(idx) = out.stdout.find(':') {
                return Ok(out.stdout[idx + 1..].trim().to_string());
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let out = exec.run("nmcli", &["-t", "-f", "ACTIVE,SSID", "dev", "wifi"]).await?;
        if out.status == 0 {
            for line in out.stdout.lines() {
                if let Some((active, ssid)) = line.split_once(':') {
                    if active == "yes" {
                        return Ok(ssid.to_string());
                    }
                }
            }
        }
    }
    Ok(String::new())
}

async fn log_vpn_packet_stats(exec: &dyn vpn_core::CommandExecutor, profile_name: &str) -> Result<(), vpn_core::CoreError> {
    #[cfg(target_os = "windows")]
    {
        // Get VPN adapter stats using PowerShell
        let script = format!(
            r#"$vpn = Get-VpnConnection -Name '{}' -ErrorAction SilentlyContinue; if ($vpn) {{ $adapter = Get-NetAdapter | Where-Object {{ $_.InterfaceIndex -eq $vpn.InterfaceIndex }}; if ($adapter) {{ $stats = Get-NetAdapterStatistics -Name $adapter.Name; Write-Host \"[VPN-PACKET] Profile: {} | Bytes RX: $($stats.BytesReceived) | Bytes TX: $($stats.BytesSent) | Packets RX: $($stats.PacketsReceived) | Packets TX: $($stats.PacketsSent)\"; }} else {{ Write-Host \"[VPN-PACKET] No adapter found for profile: {}\"; }} }} else {{ Write-Host \"[VPN-PACKET] VPN connection '{}' not found\"; }}"#,
            profile_name, profile_name, profile_name, profile_name
        );
        let args = ["-NoProfile", "-NonInteractive", "-Command", &script];
        let out = exec.run("powershell", &args).await?;
        if out.status == 0 {
            for line in out.stdout.lines() {
                if line.contains("[VPN-PACKET]") {
                    println!("{}", line);
                    eprintln!("{}", line);
                }
            }
        }
        
        // Also check rasdial for connection details
        let args = ["rasdial", profile_name];
        let out = exec.run(args[0], &args[1..]).await?;
        if out.status == 0 && out.stdout.contains("Connected to") {
            for line in out.stdout.lines() {
                if line.contains("Bytes") || line.contains("packet") || line.contains("error") || line.contains("Error") {
                    println!("[VPN-PACKET] {} - {}", profile_name, line.trim());
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // For other platforms, use similar approach
        let _ = exec;
        let _ = profile_name;
    }
    Ok(())
}

async fn set_mtu(exec: &dyn vpn_core::CommandExecutor, iface: &str, mtu: u32) -> Result<(), vpn_core::CoreError> {
    #[cfg(target_os = "windows")]
    {
        let args = [
            "interface",
            "ipv4",
            "set",
            "subinterface",
            &format!("\"{}\"", iface),
            &format!("mtu={}", mtu),
            "store=active",
        ];
        let out = exec.run("netsh", &args).await?;
        if out.status != 0 {
            return Err(vpn_core::CoreError::CommandFailed(out.stderr));
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = exec.run("ifconfig", &[iface, "mtu", &mtu.to_string()]).await?;
        if out.status != 0 {
            return Err(vpn_core::CoreError::CommandFailed(out.stderr));
        }
    }
    #[cfg(target_os = "linux")]
    {
        let out = exec.run("ip", &["link", "set", "dev", iface, "mtu", &mtu.to_string()]).await?;
        if out.status != 0 {
            return Err(vpn_core::CoreError::CommandFailed(out.stderr));
        }
    }
    Ok(())
}

#[tauri::command]
async fn daemon_spawn() -> Result<(), String> {
    // Attempts to start the desktop-service sidecar
    let (rx, child) = TauriCommand::new_sidecar("desktop-service")
        .map_err(|e| e.to_string())?
        .spawn()
        .map_err(|e| e.to_string())?;
    // detach child; listen in background for exit without blocking
    tauri::async_runtime::spawn(async move {
        let mut rx = rx;
        while let Some(_event) = rx.recv().await {
            // drain events to keep the channel from filling
        }
        let _ = child;
    });
    Ok(())
}

#[tauri::command]
async fn daemon_health() -> Result<bool, String> {
    let url = format!("http://127.0.0.1:{}/healthz", std::env::var("DESKTOP_SERVICE_PORT").ok().unwrap_or_else(|| "17231".to_string()));
    let client = reqwest::Client::new();
    match client.get(url).send().await {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
async fn daemon_start_autoconnect(args: AutoConnectArgs) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/autoconnect/start", std::env::var("DESKTOP_SERVICE_PORT").ok().unwrap_or_else(|| "17231".to_string()));
    let client = reqwest::Client::new();
    let resp = client.post(url).json(&args).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|_| "failed".to_string()));
    }
    Ok(())
}

#[tauri::command]
async fn daemon_stop_autoconnect() -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/autoconnect/stop", std::env::var("DESKTOP_SERVICE_PORT").ok().unwrap_or_else(|| "17231".to_string()));
    let client = reqwest::Client::new();
    let resp = client.post(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|_| "failed".to_string()));
    }
    Ok(())
}

#[cfg(feature = "updater")]
#[tauri::command]
async fn update_check(app: tauri::AppHandle) -> Result<(), String> {
    app.updater()
        .check()
        .await
        .map(|_resp| ())
        .map_err(|e| e.to_string())
}

#[cfg(not(feature = "updater"))]
#[tauri::command]
async fn update_check(_app: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            login,
            set_server_host,
            device_login_start,
            device_login_poll,
            fetch_regions,
            app_status,
            validate_mesh_selection,
            start_autoconnect,
            stop_autoconnect,
            daemon_spawn,
            daemon_health,
            daemon_start_autoconnect,
            daemon_stop_autoconnect,
            update_check
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


