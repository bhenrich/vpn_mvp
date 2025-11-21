use crate::peer_config::PeerDirective;
use anyhow::{anyhow, Context, Result};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::info;

pub struct WireguardDevice {
    iface: String,
    address: String,
    listen_port: u16,
    private_key_path: String,
    lock: Mutex<()>,
}

impl WireguardDevice {
    pub fn new(iface: String, address: String, listen_port: u16, private_key_path: String) -> Self {
        Self {
            iface,
            address,
            listen_port,
            private_key_path,
            lock: Mutex::new(()),
        }
    }

    pub async fn ensure_ready(&self) -> Result<()> {
        let _guard = self.lock.lock().await;
        self.ensure_interface().await?;
        self.configure_address().await?;
        self.configure_base().await?;
        Ok(())
    }

    pub async fn apply_peer(&self, peer: &PeerDirective) -> Result<()> {
        let _guard = self.lock.lock().await;
        self.configure_peer(peer).await
    }

    async fn ensure_interface(&self) -> Result<()> {
        let mut cmd = Command::new("ip");
        cmd.arg("link")
            .arg("add")
            .arg(&self.iface)
            .arg("type")
            .arg("wireguard");
        if let Err(err) = run_command(cmd, "ip link add").await {
            if !err.to_string().to_lowercase().contains("exist") {
                return Err(err);
            }
        } else {
            info!("created wireguard interface {}", self.iface);
        }
        Ok(())
    }

    async fn configure_address(&self) -> Result<()> {
        let mut flush = Command::new("ip");
        flush
            .arg("address")
            .arg("flush")
            .arg("dev")
            .arg(&self.iface);
        run_command(flush, "ip address flush").await?;

        let mut add = Command::new("ip");
        add.arg("address")
            .arg("add")
            .arg(&self.address)
            .arg("dev")
            .arg(&self.iface);
        run_command(add, "ip address add").await?;

        let mut up = Command::new("ip");
        up.arg("link")
            .arg("set")
            .arg("dev")
            .arg(&self.iface)
            .arg("up");
        run_command(up, "ip link set up").await?;
        Ok(())
    }

    async fn configure_base(&self) -> Result<()> {
        let mut cmd = Command::new("wg");
        let port = self.listen_port.to_string();
        cmd.arg("set")
            .arg(&self.iface)
            .arg("listen-port")
            .arg(&port)
            .arg("private-key")
            .arg(&self.private_key_path);
        run_command(cmd, "wg set base").await
    }

    async fn configure_peer(&self, peer: &PeerDirective) -> Result<()> {
        let mut cmd = Command::new("wg");
        cmd.arg("set")
            .arg(&self.iface)
            .arg("peer")
            .arg(&peer.public_key);
        if peer.remove {
            cmd.arg("remove");
            return run_command(cmd, "wg peer remove").await;
        }
        if !peer.allowed_ips.is_empty() {
            let allowed = peer.allowed_ips.join(",");
            cmd.arg("allowed-ips").arg(allowed);
        }
        if let Some(interval) = peer.persistent_keepalive {
            cmd.arg("persistent-keepalive").arg(interval.to_string());
        }
        run_command(cmd, "wg set peer").await
    }
}

async fn run_command(mut cmd: Command, context: &str) -> Result<()> {
    let output = cmd
        .output()
        .await
        .with_context(|| format!("failed to spawn {}", context))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("command {} failed: {}", context, stderr.trim()));
    }
    Ok(())
}
