use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use vpn_core::wg::{Peer, SessionManager, WgDeviceConfig};
use vpn_core::{CommandExecutor, CoreError};

use crate::{DnsOverride, KillSwitch, SplitTunnelRoutes};

#[derive(Debug, Clone, Default)]
pub struct SessionPolicy {
    pub interface: String,
    pub dns_servers: Vec<String>,
    pub routes_v4: Vec<String>,
    pub routes_v6: Vec<String>,
    pub kill_switch_allow_v4: Vec<String>,
    pub kill_switch_allow_v6: Vec<String>,
    pub kill_switch_allow_uids: Vec<u32>,
    pub mtu: Option<u32>,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
struct ActiveSession {
    routes_v4: Vec<String>,
    routes_v6: Vec<String>,
    peer_keys: Vec<String>,
}

pub struct WireguardSessionManager {
    #[cfg(target_os = "linux")]
    exec: Arc<dyn CommandExecutor>,
    kill_switch: KillSwitch,
    dns: DnsOverride,
    routes: SplitTunnelRoutes,
    policy: SessionPolicy,
    state: Mutex<Option<ActiveSession>>,
}

impl WireguardSessionManager {
    pub fn new(exec: Arc<dyn CommandExecutor>, policy: SessionPolicy) -> Self {
        let kill_switch = KillSwitch::new(exec.clone());
        let dns = DnsOverride::new(exec.clone());
        let routes = SplitTunnelRoutes::new(exec.clone());
        Self {
            #[cfg(target_os = "linux")]
            exec,
            kill_switch,
            dns,
            routes,
            policy,
            state: Mutex::new(None),
        }
    }

    fn iface(&self) -> &str {
        self.policy.interface.as_str()
    }
}

#[async_trait]
impl SessionManager for WireguardSessionManager {
    async fn start(&self, cfg: &WgDeviceConfig) -> Result<(), CoreError> {
        if cfg.peers.is_empty() {
            return Err(CoreError::InvalidInput(
                "WireGuard config missing peers".into(),
            ));
        }
        if cfg.name != self.policy.interface {
            return Err(CoreError::InvalidInput(format!(
                "config interface {} does not match policy {}",
                cfg.name, self.policy.interface
            )));
        }

        let guard = self.state.lock().await;
        if guard.is_some() {
            return Err(CoreError::AlreadyExists("session already active".into()));
        }

        #[cfg(not(target_os = "linux"))]
        {
            drop(guard);
            let _ = cfg;
            return Err(CoreError::UnsupportedPlatform);
        }

        #[cfg(target_os = "linux")]
        {
            let mut guard = guard;
            let peer_keys =
                match linux::setup_interface(self.exec.as_ref(), cfg, self.policy.mtu).await {
                    Ok(keys) => keys,
                    Err(err) => return Err(err),
                };

            if let Err(err) = self.apply_policy().await {
                let _ = linux::teardown_interface(self.exec.as_ref(), self.iface()).await;
                return Err(err);
            }

            let active = ActiveSession {
                routes_v4: self.policy.routes_v4.clone(),
                routes_v6: self.policy.routes_v6.clone(),
                peer_keys,
            };
            *guard = Some(active);
            Ok(())
        }
    }

    async fn stop(&self) -> Result<(), CoreError> {
        let mut guard = self.state.lock().await;
        let Some(active) = guard.take() else {
            return Ok(());
        };

        let iface = self.iface();

        let route_refs_v4: Vec<&str> = active.routes_v4.iter().map(|s| s.as_str()).collect();
        if !route_refs_v4.is_empty() {
            let _ = self.routes.clear(iface, &route_refs_v4).await;
        }
        let route_refs_v6: Vec<&str> = active.routes_v6.iter().map(|s| s.as_str()).collect();
        if !route_refs_v6.is_empty() {
            let _ = self.routes.clear(iface, &route_refs_v6).await;
        }

        if !self.policy.dns_servers.is_empty() {
            let _ = self.dns.revert(iface).await;
        }

        let _ = self.kill_switch.disable().await;

        #[cfg(target_os = "linux")]
        {
            let _ = linux::teardown_interface(self.exec.as_ref(), iface).await;
        }

        Ok(())
    }

    async fn update_peers(&self, peers: &[Peer]) -> Result<(), CoreError> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = peers;
            return Err(CoreError::UnsupportedPlatform);
        }

        #[cfg(target_os = "linux")]
        {
            let mut guard = self.state.lock().await;
            let active = guard
                .as_mut()
                .ok_or_else(|| CoreError::NotFound("session not active".into()))?;
            let peer_keys =
                linux::replace_peers(self.exec.as_ref(), self.iface(), &active.peer_keys, peers)
                    .await?;
            active.peer_keys = peer_keys;
            Ok(())
        }
    }
}

impl WireguardSessionManager {
    #[cfg(target_os = "linux")]
    async fn apply_policy(&self) -> Result<(), CoreError> {
        let iface = self.iface();

        let allow_v4: Vec<&str> = self
            .policy
            .kill_switch_allow_v4
            .iter()
            .map(|s| s.as_str())
            .collect();
        let allow_v6: Vec<&str> = self
            .policy
            .kill_switch_allow_v6
            .iter()
            .map(|s| s.as_str())
            .collect();
        let _ = self.kill_switch.disable().await;
        self.kill_switch
            .enable(
                iface,
                &allow_v4,
                &allow_v6,
                &self.policy.kill_switch_allow_uids,
            )
            .await?;

        if !self.policy.dns_servers.is_empty() {
            let dns_refs: Vec<&str> = self.policy.dns_servers.iter().map(|s| s.as_str()).collect();
            if let Err(err) = self.dns.set(iface, &dns_refs).await {
                self.kill_switch.disable().await.ok();
                return Err(err);
            }
        }

        let route_refs_v4: Vec<&str> = self.policy.routes_v4.iter().map(|s| s.as_str()).collect();
        if let Err(err) = self.routes.include(iface, &route_refs_v4).await {
            self.dns.revert(iface).await.ok();
            self.kill_switch.disable().await.ok();
            return Err(err);
        }

        let route_refs_v6: Vec<&str> = self.policy.routes_v6.iter().map(|s| s.as_str()).collect();
        if let Err(err) = self.routes.include(iface, &route_refs_v6).await {
            self.routes.clear(iface, &route_refs_v4).await.ok();
            self.dns.revert(iface).await.ok();
            self.kill_switch.disable().await.ok();
            return Err(err);
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use std::io::Write;
    use tempfile::NamedTempFile;
    use vpn_core::wg::{AllowedIp, KeyPair};

    pub async fn setup_interface(
        exec: &dyn CommandExecutor,
        cfg: &WgDeviceConfig,
        mtu: Option<u32>,
    ) -> Result<Vec<String>, CoreError> {
        teardown_interface(exec, &cfg.name).await.ok();
        create_interface(exec, cfg, mtu).await?;
        configure_peers(exec, &cfg.name, &cfg.keypair, &cfg.peers).await?;
        bring_interface_up(exec, &cfg.name).await?;
        let peer_keys = cfg
            .peers
            .iter()
            .map(|peer| encode_key(&peer.public_key))
            .collect();
        Ok(peer_keys)
    }

    pub async fn teardown_interface(
        exec: &dyn CommandExecutor,
        iface: &str,
    ) -> Result<(), CoreError> {
        let down_args = vec![
            "link".to_string(),
            "set".to_string(),
            "dev".to_string(),
            iface.to_string(),
            "down".to_string(),
        ];
        let _ = exec
            .run(
                "ip",
                &down_args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            )
            .await;

        let delete_args = vec![
            "link".to_string(),
            "delete".to_string(),
            iface.to_string(),
            "type".to_string(),
            "wireguard".to_string(),
        ];
        let out = exec
            .run(
                "ip",
                &delete_args.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            )
            .await?;
        if out.status != 0 && !out.stderr.contains("Cannot find device") {
            return Err(CoreError::CommandFailed(out.stderr));
        }
        Ok(())
    }

    pub async fn replace_peers(
        exec: &dyn CommandExecutor,
        iface: &str,
        prev_keys: &[String],
        peers: &[Peer],
    ) -> Result<Vec<String>, CoreError> {
        for key in prev_keys {
            let args = vec![
                "set".to_string(),
                iface.to_string(),
                "peer".to_string(),
                key.to_string(),
                "remove".to_string(),
            ];
            let out = exec
                .run("wg", &args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .await?;
            if out.status != 0 && !out.stderr.contains("Unknown peer") {
                return Err(CoreError::CommandFailed(out.stderr));
            }
        }

        for peer in peers {
            set_peer(exec, iface, peer).await?;
        }
        Ok(peers
            .iter()
            .map(|peer| encode_key(&peer.public_key))
            .collect())
    }

    async fn create_interface(
        exec: &dyn CommandExecutor,
        cfg: &WgDeviceConfig,
        mtu: Option<u32>,
    ) -> Result<(), CoreError> {
        let add_args = vec![
            "link".to_string(),
            "add".to_string(),
            cfg.name.clone(),
            "type".to_string(),
            "wireguard".to_string(),
        ];
        run_checked(exec, "ip", &add_args).await?;

        if let Some((addr, prefix)) = cfg.address.v4 {
            let cidr = format!("{}/{}", addr, prefix);
            let addr_args = vec![
                "address".to_string(),
                "replace".to_string(),
                cidr,
                "dev".to_string(),
                cfg.name.clone(),
            ];
            run_checked(exec, "ip", &addr_args).await?;
        }

        if let Some((addr, prefix)) = cfg.address.v6 {
            let cidr = format!("{}/{}", addr, prefix);
            let addr_args = vec![
                "-6".to_string(),
                "address".to_string(),
                "replace".to_string(),
                cidr,
                "dev".to_string(),
                cfg.name.clone(),
            ];
            run_checked(exec, "ip", &addr_args).await?;
        }

        if let Some(mtu) = mtu.or(cfg.mtu) {
            let mtu_args = vec![
                "link".to_string(),
                "set".to_string(),
                "dev".to_string(),
                cfg.name.clone(),
                "mtu".to_string(),
                mtu.to_string(),
            ];
            run_checked(exec, "ip", &mtu_args).await?;
        }

        Ok(())
    }

    async fn configure_peers(
        exec: &dyn CommandExecutor,
        iface: &str,
        keypair: &KeyPair,
        peers: &[Peer],
    ) -> Result<(), CoreError> {
        if keypair.private.iter().all(|b| *b == 0) && peers.is_empty() {
            return Ok(());
        }

        if keypair.private.iter().any(|b| *b != 0) {
            let private_b64 = BASE64.encode(&keypair.private);
            let mut key_file = NamedTempFile::new()
                .map_err(|e| CoreError::Other(format!("private key temp file: {e}")))?;
            key_file
                .write_all(private_b64.as_bytes())
                .map_err(|e| CoreError::Other(format!("write private key: {e}")))?;
            let key_path = key_file.path().to_string_lossy().to_string();

            let key_args = vec![
                "set".to_string(),
                iface.to_string(),
                "private-key".to_string(),
                key_path,
            ];
            run_checked(exec, "wg", &key_args).await?;
        }

        for peer in peers {
            set_peer(exec, iface, peer).await?;
        }
        Ok(())
    }

    async fn bring_interface_up(exec: &dyn CommandExecutor, iface: &str) -> Result<(), CoreError> {
        let args = vec![
            "link".to_string(),
            "set".to_string(),
            "dev".to_string(),
            iface.to_string(),
            "up".to_string(),
        ];
        run_checked(exec, "ip", &args).await
    }

    async fn set_peer(
        exec: &dyn CommandExecutor,
        iface: &str,
        peer: &Peer,
    ) -> Result<(), CoreError> {
        let peer_key = encode_key(&peer.public_key);
        let mut args = vec![
            "set".to_string(),
            iface.to_string(),
            "peer".to_string(),
            peer_key,
        ];

        let allowed = format_allowed_ips(&peer.allowed_ips);
        if allowed.is_empty() {
            return Err(CoreError::InvalidInput("peer missing allowed IPs".into()));
        }
        args.push("allowed-ips".to_string());
        args.push(allowed);

        if let Some(endpoint) = peer.endpoint {
            args.push("endpoint".to_string());
            args.push(endpoint.to_string());
        }

        if let Some(keepalive) = peer.persistent_keepalive_secs {
            args.push("persistent-keepalive".to_string());
            args.push(keepalive.to_string());
        }

        run_checked(exec, "wg", &args).await
    }

    fn encode_key(bytes: &[u8; 32]) -> String {
        BASE64.encode(bytes)
    }

    fn format_allowed_ips(ips: &[AllowedIp]) -> String {
        let mut out = Vec::new();
        for ip in ips {
            match ip {
                AllowedIp::V4 { addr, cidr } => out.push(format!("{}/{}", addr, cidr)),
                AllowedIp::V6 { addr, cidr } => out.push(format!("{}/{}", addr, cidr)),
            }
        }
        out.join(",")
    }

    async fn run_checked(
        exec: &dyn CommandExecutor,
        program: &str,
        args: &[String],
    ) -> Result<(), CoreError> {
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let out = exec.run(program, &arg_refs).await?;
        if out.status != 0 {
            return Err(CoreError::CommandFailed(out.stderr));
        }
        Ok(())
    }
}
