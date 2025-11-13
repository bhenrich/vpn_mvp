use std::net::SocketAddr;
use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::{sync::{Mutex, oneshot}, task::JoinHandle};
use vpn_core::{CommandExecutor, ConnectionState, FileProfileStore, VpnAdapter};

#[derive(Clone)]
struct AppState {
    svc: Arc<Mutex<AutoService>>,
}

struct AutoService {
    handle: Option<JoinHandle<()>>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl AutoService {
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

#[derive(Debug, Clone, Serialize)]
struct StatusOut {
    status: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("DESKTOP_SERVICE_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(17231);
    let app_state = AppState { svc: Arc::new(Mutex::new(AutoService::new())) };
    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/status", get(status))
        .route("/autoconnect/start", post(start_autoconnect))
        .route("/autoconnect/stop", post(stop_autoconnect))
        .with_state(app_state);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let running = state.svc.lock().await.is_running();
    let s = if running { "running" } else { "stopped" };
    (StatusCode::OK, Json(StatusOut { status: s }))
}

async fn start_autoconnect(State(state): State<AppState>, Json(args): Json<AutoConnectArgs>) -> impl IntoResponse {
    let mut svc = state.svc.lock().await;
    if svc.is_running() {
        return (StatusCode::CONFLICT, "already running");
    }
    let (tx, rx) = oneshot::channel::<()>();
    #[cfg(target_os = "windows")]
    let adapter = {
        use platform_windows::WindowsAdapter;
        let exec = Arc::new(vpn_core::RealCommandExecutor);
        let creds = Arc::new(vpn_core::OsKeyringCredentialStore);
        let profiles = Arc::new(FileProfileStore::new("vpn_mvp").unwrap());
        WindowsAdapter::new(exec.clone(), creds, profiles.clone())
    };
    #[cfg(target_os = "macos")]
    let adapter = {
        use platform_macos::MacosAdapter;
        let exec = Arc::new(vpn_core::RealCommandExecutor);
        let creds = Arc::new(vpn_core::OsKeyringCredentialStore);
        let profiles = Arc::new(FileProfileStore::new("vpn_mvp").unwrap());
        MacosAdapter::new(exec.clone(), creds, profiles.clone())
    };
    #[cfg(target_os = "linux")]
    let adapter = {
        use platform_linux::LinuxNmAdapter;
        let exec = Arc::new(vpn_core::RealCommandExecutor);
        let profiles = Arc::new(FileProfileStore::new("vpn_mvp").unwrap());
        LinuxNmAdapter::new(exec.clone(), profiles.clone())
    };
    let exec = Arc::new(vpn_core::RealCommandExecutor);
    let name = args.profile.clone();
    let iface = args.iface.clone();
    let mtu = args.mtu;
    let ssids = args.trusted_ssids.clone();
    let interval = args.interval;
    let handle = tokio::spawn(async move {
        loop {
            let ssid = current_ssid(&*exec).await.unwrap_or_default();
            let trusted = !ssid.is_empty() && ssids.iter().any(|t| t == &ssid);
            let status = adapter.status(&name).await.unwrap_or(ConnectionState::Unknown);
            if trusted {
                if matches!(status, ConnectionState::Connected | ConnectionState::Connecting) {
                    let _ = adapter.disconnect(&name).await;
                }
            } else {
                if !matches!(status, ConnectionState::Connected | ConnectionState::Connecting) {
                    let _ = adapter.connect(&name).await;
                    if let (Some(iface), Some(mtu)) = (&iface, mtu) {
                        let _ = set_mtu(&*exec, iface, mtu).await;
                    }
                }
            }
            let sleep = tokio::time::sleep(std::time::Duration::from_secs(interval));
            tokio::select! {
                _ = sleep => {},
                _ = rx => { break; }
            }
        }
    });
    svc.handle = Some(handle);
    svc.stop_tx = Some(tx);
    (StatusCode::OK, "started")
}

async fn stop_autoconnect(State(state): State<AppState>) -> impl IntoResponse {
    let mut svc = state.svc.lock().await;
    if let Some(tx) = svc.stop_tx.take() {
        let _ = tx.send(());
    }
    if let Some(handle) = svc.handle.take() {
        let _ = handle.abort();
    }
    (StatusCode::OK, "stopped")
}

async fn current_ssid(exec: &dyn CommandExecutor) -> Result<String, vpn_core::CoreError> {
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

async fn set_mtu(exec: &dyn CommandExecutor, iface: &str, mtu: u32) -> Result<(), vpn_core::CoreError> {
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


