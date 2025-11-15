use anyhow::{anyhow, Context};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand_core::OsRng;
use std::{fs, path::Path, sync::Arc};
use tokio::{
    signal,
    time::{sleep, Duration},
};
use tracing::{debug, info, warn};
use url::Url;

pub mod directory {
    tonic::include_proto!("directory.v1");
}
use directory::{directory_service_client::DirectoryServiceClient, NodeIdentity};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};

mod admission;
mod config;
mod directory_http;
mod peer_config;
mod wireguard;

use crate::wireguard::WireguardDevice;
use config::AgentConfig;
use directory_http::{DirectoryHttpClient, NodeHeartbeatRequest, NodeRegisterRequest};

async fn connect_directory(
    address: &str,
) -> Result<DirectoryServiceClient<Channel>, anyhow::Error> {
    // Read env to determine if mTLS should be used
    let mtls_enabled = std::env::var("MTLS_ENABLED").unwrap_or_else(|_| "0".to_string()) == "1";
    let endpoint = Endpoint::from_shared(address.to_string())
        .with_context(|| format!("invalid DIRECTORY_GRPC_ADDR: {}", address))?;

    if mtls_enabled {
        let url =
            Url::parse(address).with_context(|| "failed to parse DIRECTORY_GRPC_ADDR for mTLS")?;
        let domain = url
            .host_str()
            .ok_or_else(|| anyhow!("missing host in DIRECTORY_GRPC_ADDR"))?;

        let ca_path =
            std::env::var("MTLS_CA_CERT_PATH").with_context(|| "MTLS_CA_CERT_PATH not set")?;
        let ca_pem = std::fs::read(&ca_path)
            .with_context(|| format!("failed to read CA cert at {}", ca_path))?;

        let client_cert_path = std::env::var("MTLS_CLIENT_CERT_PATH")
            .with_context(|| "MTLS_CLIENT_CERT_PATH not set")?;
        let client_key_path = std::env::var("MTLS_CLIENT_KEY_PATH")
            .with_context(|| "MTLS_CLIENT_KEY_PATH not set")?;
        let client_cert_pem = std::fs::read(&client_cert_path)
            .with_context(|| format!("failed to read client cert at {}", client_cert_path))?;
        let client_key_pem = std::fs::read(&client_key_path)
            .with_context(|| format!("failed to read client key at {}", client_key_path))?;

        let tls = ClientTlsConfig::new()
            .domain_name(domain.to_string())
            .ca_certificate(Certificate::from_pem(ca_pem))
            .identity(Identity::from_pem(client_cert_pem, client_key_pem));

        let channel = endpoint.tls_config(tls)?.connect().await?;
        info!("directory client connected with mTLS");
        return Ok(DirectoryServiceClient::new(channel));
    }

    // Non-mTLS (dev) path
    info!("directory client connecting without mTLS (dev)");
    let channel = endpoint.connect().await?;
    Ok(DirectoryServiceClient::new(channel))
}

async fn config_stream_task(
    mut client: DirectoryServiceClient<Channel>,
    identity: NodeIdentity,
    wg: Arc<WireguardDevice>,
) {
    loop {
        match client.stream_config(identity.clone()).await {
            Ok(mut stream) => {
                info!("connected to config stream");
                while let Some(update) = stream.get_mut().message().await.transpose() {
                    match update {
                        Ok(update) => match peer_config::parse_peer_config(&update.wg_config) {
                            Ok(peer) => {
                                if let Err(err) = wg.apply_peer(&peer).await {
                                    warn!(
                                        "failed to apply peer {} (rev {}): {}",
                                        peer.public_key, update.revision, err
                                    );
                                } else {
                                    debug!(
                                        "applied peer {} allowed_ips={:?}",
                                        peer.public_key, peer.allowed_ips
                                    );
                                }
                            }
                            Err(err) => warn!("invalid peer config payload: {}", err),
                        },
                        Err(err) => {
                            warn!("config stream error: {}", err);
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                warn!("stream_config connect failed: {}", err);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();
    info!("node-agent starting (skeleton)");

    let wg_key_path = std::env::var("WG_SERVER_KEY_PATH")
        .unwrap_or_else(|_| "/var/run/vpn/server.key".to_string());
    let server_keys = ensure_server_keys(&wg_key_path)?;
    info!("using WireGuard public key {}", server_keys.public_key_b64);

    let agent_config = AgentConfig::from_env()?;
    info!(
        "using directory HTTP {} and gRPC {}",
        agent_config.http_addr, agent_config.grpc_addr
    );

    let directory_http = DirectoryHttpClient::new(&agent_config.http_addr)?;
    let region_id =
        if let Some(id) = agent_config.region_id.clone() {
            id
        } else {
            let code = agent_config.region_code.as_deref().ok_or_else(|| {
                anyhow!("NODE_REGION_CODE must be set if NODE_REGION_ID is missing")
            })?;
            let city = agent_config.region_city.as_deref().ok_or_else(|| {
                anyhow!("NODE_REGION_CITY must be set if NODE_REGION_ID is missing")
            })?;
            directory_http.resolve_region_id(code, city).await?
        };

    let internal_wg_ip = agent_config.wg_internal_ip()?;
    let register_payload = NodeRegisterRequest {
        public_key: server_keys.public_key_b64.clone(),
        region_id: Some(region_id.clone()),
        internal_wg_ip: Some(internal_wg_ip.clone()),
        egress_ips: if agent_config.egress_ips.is_empty() {
            None
        } else {
            Some(agent_config.egress_ips.clone())
        },
        public_endpoint: Some(agent_config.public_endpoint.clone()),
        listen_port: Some(agent_config.listen_port),
        capabilities: None,
        agent_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        wg_rs_version: Some("kernel-wgtools".to_string()),
    };
    directory_http.register_node(&register_payload).await?;
    info!(
        "registered node for region {} with endpoint {}:{}",
        region_id, agent_config.public_endpoint, agent_config.listen_port
    );

    let wireguard = Arc::new(WireguardDevice::new(
        agent_config.wg_interface.clone(),
        agent_config.wg_address.clone(),
        agent_config.listen_port,
        wg_key_path.clone(),
    ));
    wireguard.ensure_ready().await?;
    info!("wireguard interface ready");

    let node_identity = NodeIdentity {
        public_key: server_keys.public_key_b64.clone(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        wg_rs_version: "kernel-wgtools".to_string(),
    };

    {
        let identity = node_identity.clone();
        let wg = wireguard.clone();
        let grpc_addr = agent_config.grpc_addr.clone();
        let client_for_stream = connect_directory(&grpc_addr)
            .await
            .with_context(|| format!("config stream connect failed ({})", grpc_addr))?;
        tokio::spawn(async move { config_stream_task(client_for_stream, identity, wg).await });
    }

    {
        let http = directory_http.clone();
        let heartbeat_key = server_keys.public_key_b64.clone();
        tokio::spawn(async move {
            let agent_version = env!("CARGO_PKG_VERSION").to_string();
            let wg_rs_version = "kernel-wgtools".to_string();
            loop {
                let payload = NodeHeartbeatRequest {
                    public_key: heartbeat_key.clone(),
                    status: Some("online".to_string()),
                    agent_version: Some(agent_version.clone()),
                    wg_rs_version: Some(wg_rs_version.clone()),
                };
                if let Err(err) = http.send_heartbeat(&payload).await {
                    warn!("heartbeat failed: {}", err);
                }
                sleep(Duration::from_secs(30)).await;
            }
        });
    }

    let mut shutdown = Box::pin(signal::ctrl_c());

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("shutdown signal received");
                break;
            }
            _ = async {
                if std::env::var("ENABLE_ADMISSION").unwrap_or_else(|_| "0".to_string()) == "1" {
                    if let Err(err) = admission::run_admission_server().await {
                        warn!("admission server error: {}", err);
                    }
                }
            } => {}
        }
    }

    info!("node-agent shutting down");
    Ok(())
}

struct ServerKeys {
    public_key_b64: String,
}

fn ensure_server_keys(path: &str) -> Result<ServerKeys, anyhow::Error> {
    let key_path = Path::new(path);
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create key directory {}", parent.display()))?;
    }

    if !key_path.exists() {
        let secret = x25519_dalek::StaticSecret::new(OsRng);
        let priv_b64 = BASE64.encode(secret.to_bytes());
        let public = x25519_dalek::PublicKey::from(&secret);
        let pub_b64 = BASE64.encode(public.as_bytes());
        fs::write(key_path, format!("{}\n", priv_b64))
            .with_context(|| "failed to write server private key")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(key_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(key_path, perms)?;
        }
        return Ok(ServerKeys {
            public_key_b64: pub_b64,
        });
    }

    let priv_b64 = fs::read_to_string(key_path)
        .with_context(|| "failed to read server private key")?
        .trim()
        .to_string();
    let priv_bytes = BASE64
        .decode(priv_b64.as_bytes())
        .with_context(|| "invalid base64 private key")?;
    if priv_bytes.len() != 32 {
        return Err(anyhow!("private key must be 32 bytes"));
    }
    let mut secret_bytes = [0u8; 32];
    secret_bytes.copy_from_slice(&priv_bytes);
    let secret = x25519_dalek::StaticSecret::from(secret_bytes);
    let public = x25519_dalek::PublicKey::from(&secret);
    let pub_b64 = BASE64.encode(public.as_bytes());
    Ok(ServerKeys {
        public_key_b64: pub_b64,
    })
}
