//! Desktop core crate
//!
//! This crate provides a high-level, cross-platform API for the desktop
//! VPN client. It wraps the lower-level `vpn-core` primitives and the
//! per-OS `platform-*` crates to offer:
//!
//! - TUN lifecycle management (bring-up/down, MTU, addressing).
//! - Kill switch enable/disable per OS.
//! - DNS override and leak protection helpers.
//! - Route-based split tunnelling helpers.
//! - An auto-connect loop with trusted networks and MTU tuning.
//!
//! It is intended to be used by the background desktop service and
//! the Tauri UI via a thin IPC layer.

use std::sync::Arc;

use tokio::{sync::oneshot, task::JoinHandle};
use vpn_core::{
    ConnectionState, CoreError, DeviceAddress, TunBackend, VpnAdapter, CommandExecutor,
};

pub use vpn_core::wg::{AllowedIp, KeyPair, Peer, WgDeviceConfig};

/// High-level configuration for the auto-connect background loop.
#[derive(Debug, Clone)]
pub struct AutoConnectConfig {
    /// Profile name to connect/disconnect.
    pub profile: String,
    /// Optional interface name for MTU adjustment on connect.
    pub iface: Option<String>,
    /// Optional MTU value to set when the interface is connected.
    pub mtu: Option<u32>,
    /// SSIDs that should be treated as trusted (stay disconnected).
    pub trusted_ssids: Vec<String>,
    /// Poll interval in seconds.
    pub interval_secs: u64,
}

/// Handle for a running auto-connect task.
#[derive(Debug)]
pub struct AutoConnectHandle {
    handle: JoinHandle<()>,
    stop_tx: oneshot::Sender<()>,
}

impl AutoConnectHandle {
    /// Signal the task to stop and abort the join handle (fire-and-forget).
    pub fn stop(self) {
        let _ = self.stop_tx.send(());
        self.handle.abort();
    }
}

/// Spawn an auto-connect background loop.
///
/// The loop:
/// - Polls the current Wi‑Fi SSID.
/// - Connects when *not* on a trusted SSID.
/// - Disconnects when *on* a trusted SSID.
/// - Optionally sets MTU on the given interface when connecting.
///
/// The returned handle can be used to stop the loop.
pub fn spawn_autoconnect<T>(
    exec: Arc<dyn CommandExecutor>,
    adapter: T,
    cfg: AutoConnectConfig,
) -> AutoConnectHandle
where
    T: VpnAdapter + Send + Sync + 'static,
{
    let (tx, mut rx) = oneshot::channel::<()>();
    let AutoConnectConfig {
        profile,
        iface,
        mtu,
        trusted_ssids,
        interval_secs,
    } = cfg;

    let handle = tokio::spawn(async move {
        loop {
            let ssid = match current_ssid(exec.as_ref()).await {
                Ok(s) => s,
                Err(_) => String::new(),
            };
            let trusted = !ssid.is_empty() && trusted_ssids.iter().any(|t| t == &ssid);
            let status = adapter
                .status(&profile)
                .await
                .unwrap_or(ConnectionState::Unknown);

            if trusted {
                if matches!(status, ConnectionState::Connected | ConnectionState::Connecting) {
                    let _ = adapter.disconnect(&profile).await;
                }
            } else if !matches!(status, ConnectionState::Connected | ConnectionState::Connecting) {
                let _ = adapter.connect(&profile).await;
                if let (Some(iface), Some(mtu)) = (&iface, mtu) {
                    let _ = set_mtu(exec.as_ref(), iface, mtu).await;
                }
            }

            let sleep = tokio::time::sleep(std::time::Duration::from_secs(interval_secs));
            tokio::select! {
                _ = sleep => {},
                _ = &mut rx => {
                    break;
                }
            }
        }
    });

    AutoConnectHandle { handle, stop_tx: tx }
}

/// Cross-platform helper to query the current Wi‑Fi SSID, if any.
pub async fn current_ssid(exec: &dyn CommandExecutor) -> Result<String, CoreError> {
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
        let out = exec
            .run("networksetup", &["-getairportnetwork", "Wi-Fi"])
            .await?;
        if out.status == 0 {
            if let Some(idx) = out.stdout.find(':') {
                return Ok(out.stdout[idx + 1..].trim().to_string());
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let out = exec
            .run("nmcli", &["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
            .await?;
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

/// Cross-platform helper to set MTU on a given interface.
pub async fn set_mtu(exec: &dyn CommandExecutor, iface: &str, mtu: u32) -> Result<(), CoreError> {
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
            return Err(CoreError::CommandFailed(out.stderr));
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = exec
            .run("ifconfig", &[iface, "mtu", &mtu.to_string()])
            .await?;
        if out.status != 0 {
            return Err(CoreError::CommandFailed(out.stderr));
        }
    }
    #[cfg(target_os = "linux")]
    {
        let out = exec
            .run("ip", &["link", "set", "dev", iface, "mtu", &mtu.to_string()])
            .await?;
        if out.status != 0 {
            return Err(CoreError::CommandFailed(out.stderr));
        }
    }
    Ok(())
}

/// Platform-agnostic wrapper around the per-OS TUN implementations.
///
/// This uses the `TunBackend` trait from `vpn-core::wg` and delegates
/// to the concrete backend provided by each `platform-*` crate.
pub struct PlatformTun {
    inner: Arc<dyn TunBackend>,
}

impl PlatformTun {
    /// Construct a new `PlatformTun` for the given interface name.
    pub fn new(exec: Arc<dyn CommandExecutor>, iface: String) -> Self {
        #[cfg(target_os = "windows")]
        let inner = {
            use platform_windows::tun_backend::WindowsWintunBackend;
            Arc::new(WindowsWintunBackend::new(exec, iface)) as Arc<dyn TunBackend>
        };
        #[cfg(target_os = "macos")]
        let inner = {
            use platform_macos::tun_backend::MacOsUtunBackend;
            Arc::new(MacOsUtunBackend::new(exec, iface)) as Arc<dyn TunBackend>
        };
        #[cfg(target_os = "linux")]
        let inner = {
            use platform_linux::tun_backend::LinuxIpTunBackend;
            Arc::new(LinuxIpTunBackend::new(exec, iface)) as Arc<dyn TunBackend>
        };

        Self { inner }
    }

    /// Open (create if necessary) the interface.
    pub async fn open(&self, name: &str) -> Result<(), CoreError> {
        self.inner.open(name).await
    }

    /// Bring the interface up.
    pub async fn up(&self) -> Result<(), CoreError> {
        self.inner.up().await
    }

    /// Bring the interface down.
    pub async fn down(&self) -> Result<(), CoreError> {
        self.inner.down().await
    }

    /// Set MTU on the interface.
    pub async fn set_mtu(&self, mtu: u32) -> Result<(), CoreError> {
        self.inner.set_mtu(mtu).await
    }

    /// Configure IPv4/IPv6 addresses on the interface.
    pub async fn set_address(&self, address: &DeviceAddress) -> Result<(), CoreError> {
        self.inner.set_address(address).await
    }
}

/// Cross-platform kill switch operations.
pub struct KillSwitch {
    exec: Arc<dyn CommandExecutor>,
}

impl KillSwitch {
    pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
        Self { exec }
    }

    /// Apply a fail-closed kill switch for the given interface.
    ///
    /// - `iface` is the TUN interface (or alias) to allow.
    /// - `allow_v4` / `allow_v6` are control-plane IPs that must bypass the switch.
    /// - `allow_uids` is only used on Linux for UID-based bypass.
    pub async fn enable(
        &self,
        iface: &str,
        allow_v4: &[&str],
        allow_v6: &[&str],
        _allow_uids: &[u32],
    ) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        {
            use platform_windows::firewall::WindowsFirewall;
            let fw = WindowsFirewall::new(self.exec.clone());
            fw.apply_killswitch(iface, allow_v4, allow_v6).await?;
        }
        #[cfg(target_os = "macos")]
        {
            use platform_macos::firewall::MacPfFirewall;
            let fw = MacPfFirewall::new(self.exec.clone());
            fw.apply_killswitch(iface, allow_v4, allow_v6).await?;
        }
        #[cfg(target_os = "linux")]
        {
            use platform_linux::firewall::LinuxNftables;
            let fw = LinuxNftables::new(self.exec.clone());
            if _allow_uids.is_empty() {
                fw.apply_killswitch(iface, allow_v4, allow_v6).await?;
            } else {
                fw.apply_killswitch_with_uids(iface, allow_v4, allow_v6, _allow_uids)
                    .await?;
            }
        }
        Ok(())
    }

    /// Remove the kill switch rules (best-effort).
    pub async fn disable(&self) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        {
            use platform_windows::firewall::WindowsFirewall;
            let fw = WindowsFirewall::new(self.exec.clone());
            fw.remove_killswitch().await?;
        }
        #[cfg(target_os = "macos")]
        {
            use platform_macos::firewall::MacPfFirewall;
            let fw = MacPfFirewall::new(self.exec.clone());
            fw.revert().await?;
        }
        #[cfg(target_os = "linux")]
        {
            use platform_linux::firewall::LinuxNftables;
            let fw = LinuxNftables::new(self.exec.clone());
            fw.remove_killswitch().await?;
        }
        Ok(())
    }
}

/// DNS override and leak-protection helpers.
pub struct DnsOverride {
    exec: Arc<dyn CommandExecutor>,
}

impl DnsOverride {
    pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
        Self { exec }
    }

    /// Set DNS for the given interface/service.
    ///
    /// - Windows: `iface` is an interface alias.
    /// - macOS: `iface` is a network service (e.g., "Wi-Fi").
    /// - Linux: `iface` is a link name (e.g., "tun0").
    pub async fn set(&self, iface: &str, servers: &[&str]) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        {
            use platform_windows::dns::WindowsDns;
            let dns = WindowsDns::new(self.exec.clone());
            dns.set_for_interface(iface, servers).await?;
        }
        #[cfg(target_os = "macos")]
        {
            use platform_macos::dns::MacDns;
            let dns = MacDns::new(self.exec.clone());
            dns.set_dns_for_service(iface, servers).await?;
        }
        #[cfg(target_os = "linux")]
        {
            use platform_linux::dns::LinuxResolvedDns;
            let dns = LinuxResolvedDns::new(self.exec.clone());
            dns.set_dns(iface, servers).await?;
        }
        Ok(())
    }

    /// Revert DNS for the given interface/service to system defaults (best-effort).
    pub async fn revert(&self, iface: &str) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        {
            use platform_windows::dns::WindowsDns;
            let dns = WindowsDns::new(self.exec.clone());
            dns.set_for_interface(iface, &[]).await?;
        }
        #[cfg(target_os = "macos")]
        {
            use platform_macos::dns::MacDns;
            let dns = MacDns::new(self.exec.clone());
            dns.set_dns_for_service(iface, &[]).await?;
        }
        #[cfg(target_os = "linux")]
        {
            use platform_linux::dns::LinuxResolvedDns;
            let dns = LinuxResolvedDns::new(self.exec.clone());
            dns.revert(iface).await?;
        }
        Ok(())
    }
}

/// Route-based split tunnelling helpers.
pub struct SplitTunnelRoutes {
    exec: Arc<dyn CommandExecutor>,
}

impl SplitTunnelRoutes {
    pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
        Self { exec }
    }

    /// Include only the given CIDRs via the VPN interface.
    pub async fn include(&self, iface: &str, cidrs: &[&str]) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        {
            use platform_windows::split::WindowsSplitRoutes;
            let split = WindowsSplitRoutes::new(self.exec.clone());
            split.set_include(iface, cidrs).await?;
        }
        #[cfg(target_os = "macos")]
        {
            use platform_macos::split::MacSplitRoutes;
            let split = MacSplitRoutes::new(self.exec.clone());
            split.set_include(iface, cidrs).await?;
        }
        #[cfg(target_os = "linux")]
        {
            use platform_linux::split::LinuxSplitRoutes;
            let split = LinuxSplitRoutes::new(self.exec.clone());
            split.set_include(iface, cidrs).await?;
        }
        Ok(())
    }

    /// Clear routing rules previously applied by `include`.
    pub async fn clear(&self, iface: &str, cidrs: &[&str]) -> Result<(), CoreError> {
        #[cfg(target_os = "windows")]
        {
            use platform_windows::split::WindowsSplitRoutes;
            let split = WindowsSplitRoutes::new(self.exec.clone());
            split.clear(iface, cidrs).await?;
        }
        #[cfg(target_os = "macos")]
        {
            use platform_macos::split::MacSplitRoutes;
            let split = MacSplitRoutes::new(self.exec.clone());
            split.clear(iface, cidrs).await?;
        }
        #[cfg(target_os = "linux")]
        {
            use platform_linux::split::LinuxSplitRoutes;
            let split = LinuxSplitRoutes::new(self.exec.clone());
            split.clear(iface, cidrs).await?;
        }
        Ok(())
    }
}


