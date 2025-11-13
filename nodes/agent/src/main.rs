use anyhow::{Context, anyhow};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use rand_core::OsRng;
use tokio::{signal, time::{sleep, Duration}};
use tracing::{error, info, warn};
use url::Url;

pub mod directory {
	tonic::include_proto!("directory.v1");
}
use directory::{directory_service_client::DirectoryServiceClient, HeartbeatRequest, NodeIdentity};
use tonic::transport::{Channel, ClientTlsConfig, Certificate, Identity, Endpoint};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::process::Command;

mod admission;

fn generate_ephemeral_wg_keypair_base64() -> (String, x25519_dalek::StaticSecret) {
	let secret = x25519_dalek::StaticSecret::new(OsRng);
	let public = x25519_dalek::PublicKey::from(&secret);
	let public_b64 = BASE64.encode(public.as_bytes());
	(public_b64, secret)
}

async fn connect_directory(address: &str) -> Result<DirectoryServiceClient<Channel>, anyhow::Error> {
	// Read env to determine if mTLS should be used
	let mtls_enabled = std::env::var("MTLS_ENABLED").unwrap_or_else(|_| "0".to_string()) == "1";
	let endpoint = Endpoint::from_shared(address.to_string())
		.with_context(|| format!("invalid DIRECTORY_ADDR: {}", address))?;

	if mtls_enabled {
		let url = Url::parse(address).with_context(|| "failed to parse DIRECTORY_ADDR for mTLS")?;
		let domain = url.host_str().ok_or_else(|| anyhow!("missing host in DIRECTORY_ADDR"))?;

		let ca_path = std::env::var("MTLS_CA_CERT_PATH")
			.with_context(|| "MTLS_CA_CERT_PATH not set")?;
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

async fn rotate_keys_task(identity: Arc<RwLock<NodeIdentity>>) {
	let period_secs: u64 = std::env::var("KEY_ROTATION_SECS")
		.ok()
		.and_then(|v| v.parse().ok())
		.unwrap_or(24 * 60 * 60);
	loop {
		sleep(Duration::from_secs(period_secs)).await;
		let (public_key_b64, _secret) = generate_ephemeral_wg_keypair_base64();
		{
			let mut id = identity.write().await;
			id.public_key = public_key_b64.clone();
		}
		info!("rotated ephemeral WireGuard key");
	}
}

async fn config_stream_task(mut client: DirectoryServiceClient<Channel>, identity: Arc<RwLock<NodeIdentity>>) {
	loop {
		let id = { identity.read().await.clone() };
		match client.stream_config(id).await {
			Ok(mut stream) => {
				info!("connected to config stream");
				use tokio_stream::StreamExt;
				while let Some(update) = stream.get_mut().message().await.transpose() {
					match update {
						Ok(update) => {
							let path = std::env::var("WG_CONFIG_PATH").unwrap_or_else(|_| "/var/run/vpn/wg0.conf".to_string());
							if let Err(e) = std::fs::create_dir_all("/var/run/vpn") {
								warn!("failed to ensure /var/run/vpn: {}", e);
							}
							match std::fs::write(&path, &update.wg_config) {
								Ok(_) => {
                                    info!("received config revision {} and wrote {}", update.revision, path);
									if let Ok(cmd) = std::env::var("APPLY_WG_CONFIG_CMD") {
										if !cmd.is_empty() {
											let mut parts = cmd.split_whitespace();
											if let Some(bin) = parts.next() {
												let args: Vec<&str> = parts.collect();
												let status = Command::new(bin).args(args).status().await;
												match status {
													Ok(exit) => info!("apply command exited: {}", exit),
													Err(err) => warn!("apply command failed to spawn: {}", err),
												}
											}
										}
									}
								}
								Err(err) => warn!("failed writing config: {}", err),
							}
						}
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

	// Ephemeral WG keypair (RAM-only)
	let (public_key_b64, _secret) = generate_ephemeral_wg_keypair_base64();
	info!("generated ephemeral WireGuard public key");

	// Directory service address (plaintext dev default). Example: http://127.0.0.1:8081
	let directory_addr = std::env::var("DIRECTORY_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
	info!("using directory service at {}", directory_addr);

	// Connect gRPC client
	let mut client = connect_directory(&directory_addr).await
		.with_context(|| format!("failed connecting to directory at {}", directory_addr))?;

	// No-op WG bring-up placeholder (handled via APPLY_WG_CONFIG_CMD when config arrives)
	info!("wireguard-rs bring-up handled via APPLY_WG_CONFIG_CMD when config arrives");

	// Heartbeat loop
	let heartbeat_identity = Arc::new(RwLock::new(NodeIdentity {
		public_key: public_key_b64.clone(),
		agent_version: env!("CARGO_PKG_VERSION").to_string(),
		wg_rs_version: "n/a".to_string(),
	}));

	// Spawn key rotation task
	{
		let id = heartbeat_identity.clone();
		tokio::spawn(async move { rotate_keys_task(id).await });
	}

	// Spawn config stream task
	{
		let id = heartbeat_identity.clone();
		let client_for_stream = connect_directory(&directory_addr).await
			.with_context(|| "config stream connect failed")?;
		tokio::spawn(async move { config_stream_task(client_for_stream, id).await });
	}

	let mut shutdown = signal::ctrl_c();

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
				let id = { heartbeat_identity.read().await.clone() };
				let req = HeartbeatRequest {
					identity: Some(id),
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


