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
use directory::{directory_service_client::DirectoryServiceClient, HeartbeatRequest, NodeIdentity};
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};

mod admission;
mod peer_config;
mod wireguard;

use crate::wireguard::WireguardDevice;

async fn connect_directory(
    address: &str,
) -> Result<DirectoryServiceClient<Channel>, anyhow::Error> {
    // Read env to determine if mTLS should be used
    let mtls_enabled = std::env::var("MTLS_ENABLED").unwrap_or_else(|_| "0".to_string()) == "1";
    let endpoint = Endpoint::from_shared(address.to_string())
        .with_context(|| format!("invalid DIRECTORY_ADDR: {}", address))?;

    if mtls_enabled {
        let url = Url::parse(address).with_context(|| "failed to parse DIRECTORY_ADDR for mTLS")?;
        let domain = url
            .host_str()
            .ok_or_else(|| anyhow!("missing host in DIRECTORY_ADDR"))?;

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

    // Directory service address (plaintext dev default). Example: http://127.0.0.1:8081
    let directory_addr =
        std::env::var("DIRECTORY_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
    info!("using directory service at {}", directory_addr);

    // Connect gRPC client
    let mut client = connect_directory(&directory_addr)
        .await
        .with_context(|| format!("failed connecting to directory at {}", directory_addr))?;

    let wg_interface = std::env::var("WG_INTERFACE").unwrap_or_else(|_| "wg0".to_string());
    let wg_address = std::env::var("WG_ADDRESS").unwrap_or_else(|_| "10.66.0.1/24".to_string());
    let wg_listen_port: u16 = std::env::var("WG_LISTEN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(51820);

    let wireguard = Arc::new(WireguardDevice::new(
        wg_interface,
        wg_address,
        wg_listen_port,
        wg_key_path.clone(),
    ));
    wireguard.ensure_ready().await?;
    info!("wireguard interface ready");

    let node_identity = NodeIdentity {
        public_key: server_keys.public_key_b64.clone(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        wg_rs_version: "kernel-wgtools".to_string(),
    };

    // Spawn config stream task
    {
        let identity = node_identity.clone();
        let wg = wireguard.clone();
        let client_for_stream = connect_directory(&directory_addr)
            .await
            .with_context(|| "config stream connect failed")?;
        tokio::spawn(async move { config_stream_task(client_for_stream, identity, wg).await });
    }

    let mut shutdown = Box::pin(signal::ctrl_c());

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("shutdown signal received");
                break;
            }
            _ = async {
                // Launch admission server lazily if enabled
                if std::env::var("ENABLE_ADMISSION").unwrap_or_else(|_| "0".to_string()) == "1" {
                    match admission::run_admission_server().await {
                        Ok(()) => info!("admission server exited"),
                        Err(err) => warn!("admission server error: {}", err),
                    }
                }
            } => {}
            _ = async {
                let req = HeartbeatRequest {
                    identity: Some(node_identity.clone()),
                    status: "online".to_string(),
                };
                match client.heartbeat(req).await {
                    Ok(resp) => {
                        let inner = resp.into_inner();
                        info!("heartbeat ack: node_id={}, status={}", inner.node_id, inner.status);
                    }
                    Err(err) => {
                        warn!("heartbeat failed: {}", err);
                        // Attempt reconnect (reloads certs if mTLS enabled)
                        if let Ok(new_client) = connect_directory(&directory_addr).await {
                            client = new_client;
                            info!("reconnected directory client");
                        }
                    }
                }
                sleep(Duration::from_secs(30)).await;
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
