#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();
    daemon::run().await
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("agent-macos is only available on macOS targets");
}

#[cfg(target_os = "macos")]
mod daemon {
    use std::net::SocketAddr;
    use std::sync::Arc;

    use agent_common::{AgentError, BasicResponse, ConnectRequest, StatusResponse};
    use axum::{
        extract::State,
        http::StatusCode,
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };
    use tokio::sync::Mutex;
    use tracing::info;
    use vpn_core::{CommandExecutor, RealCommandExecutor, SessionManager};

    use desktop_core::WireguardSessionManager;

    type SharedState = Arc<Mutex<Option<SessionHandle>>>;

    #[derive(Clone)]
    pub struct AppState {
        exec: Arc<dyn CommandExecutor>,
        session: SharedState,
    }

    struct SessionHandle {
        manager: Arc<WireguardSessionManager>,
        iface: String,
    }

    pub async fn run() -> anyhow::Result<()> {
        let port: u16 = std::env::var("MACOS_AGENT_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(18082);
        let state = AppState {
            exec: Arc::new(RealCommandExecutor),
            session: Arc::new(Mutex::new(None)),
        };
        let app = Router::new()
            .route("/healthz", get(healthz))
            .route("/status", get(status))
            .route("/connect", post(connect))
            .route("/disconnect", post(disconnect))
            .with_state(state);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        info!("macos agent listening on {}", addr);
        axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
        Ok(())
    }

    async fn healthz() -> impl IntoResponse {
        (StatusCode::OK, "ok")
    }

    async fn status(State(state): State<AppState>) -> impl IntoResponse {
        let guard = state.session.lock().await;
        let (status, iface) = match guard.as_ref() {
            Some(handle) => ("connected", Some(handle.iface.clone())),
            None => ("disconnected", None),
        };
        (
            StatusCode::OK,
            Json(StatusResponse {
                status,
                interface: iface,
            }),
        )
    }

    async fn connect(
        State(state): State<AppState>,
        Json(payload): Json<ConnectRequest>,
    ) -> Result<impl IntoResponse, AgentError> {
        let mut guard = state.session.lock().await;
        if guard.is_some() {
            return Err(AgentError::Conflict("session already active".to_string()));
        }
        let (config, policy) = payload.into_session()?;
        let manager = Arc::new(WireguardSessionManager::new(state.exec.clone(), policy));
        if let Err(err) = manager.start(&config).await {
            return Err(err.into());
        }
        let handle = SessionHandle {
            iface: config.name.clone(),
            manager: manager.clone(),
        };
        *guard = Some(handle);
        Ok((
            StatusCode::OK,
            Json(BasicResponse {
                status: "connected",
            }),
        ))
    }

    async fn disconnect(State(state): State<AppState>) -> Result<impl IntoResponse, AgentError> {
        let mut guard = state.session.lock().await;
        if let Some(handle) = guard.take() {
            handle.manager.stop().await?;
        }
        Ok((
            StatusCode::OK,
            Json(BasicResponse {
                status: "disconnected",
            }),
        ))
    }
}
