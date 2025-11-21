//! Linux-specific adapter crate.
//! Contains:
//! - NetworkManager and strongSwan adapters for legacy flows.
//! - Linux TUN backend using `ip` for link management for the WireGuard core.

use std::sync::Arc;

use vpn_core::{CommandExecutor, ConnectionState, CoreError, ProfileStore, VpnAdapter, VpnProfile};

pub mod tun_backend;
pub mod firewall;
pub mod dns;
pub mod split;

pub struct LinuxNmAdapter {
	exec: Arc<dyn CommandExecutor>,
	profiles: Arc<dyn ProfileStore>,
}

impl LinuxNmAdapter {
	pub fn new(exec: Arc<dyn CommandExecutor>, profiles: Arc<dyn ProfileStore>) -> Self {
		Self { exec, profiles }
	}
}

#[async_trait::async_trait]
impl VpnAdapter for LinuxNmAdapter {
	async fn init(&self) -> Result<(), CoreError> {
		Ok(())
	}

	async fn add_profile(&self, profile: VpnProfile) -> Result<(), CoreError> {
		// MVP: store profile; provisioning via nmcli import/add to be added
		self.profiles.save(&profile).await
	}

	async fn remove_profile(&self, name: &str) -> Result<(), CoreError> {
		// Try removing any existing NM connection, but ignore errors
		let _ = self.exec.run("nmcli", &["connection", "delete", "id", name]).await;
		self.profiles.delete(name).await
	}

	async fn list_profiles(&self) -> Result<Vec<VpnProfile>, CoreError> {
		self.profiles.list().await
	}

	async fn connect(&self, name: &str) -> Result<(), CoreError> {
		let out = self.exec.run("nmcli", &["connection", "up", "id", name]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn disconnect(&self, name: &str) -> Result<(), CoreError> {
		let out = self.exec.run("nmcli", &["connection", "down", "id", name]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn status(&self, name: &str) -> Result<ConnectionState, CoreError> {
		let out = self
			.exec
			.run("nmcli", &["-t", "-f", "GENERAL.STATE", "connection", "show", name])
			.await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		let s = out.stdout;
		// Expect format like: GENERAL.STATE:activated (path:...)
		if s.contains("activated") {
			Ok(ConnectionState::Connected)
		} else if s.contains("activating") {
			Ok(ConnectionState::Connecting)
		} else if s.contains("deactivated") {
			Ok(ConnectionState::Disconnected)
		} else {
			Ok(ConnectionState::Unknown)
		}
	}
}

pub struct LinuxSwanctlAdapter {
	exec: Arc<dyn CommandExecutor>,
	profiles: Arc<dyn ProfileStore>,
}

impl LinuxSwanctlAdapter {
	pub fn new(exec: Arc<dyn CommandExecutor>, profiles: Arc<dyn ProfileStore>) -> Self {
		Self { exec, profiles }
	}
}

#[async_trait::async_trait]
impl VpnAdapter for LinuxSwanctlAdapter {
	async fn init(&self) -> Result<(), CoreError> {
		Ok(())
	}

	async fn add_profile(&self, profile: VpnProfile) -> Result<(), CoreError> {
		// Store for reference; actual swanctl provisioning requires config files loaded by charon
		self.profiles.save(&profile).await
	}

	async fn remove_profile(&self, name: &str) -> Result<(), CoreError> {
		// Best-effort terminate then remove from store
		let _ = self.exec.run("swanctl", &["--terminate", "--child", name]).await;
		self.profiles.delete(name).await
	}

	async fn list_profiles(&self) -> Result<Vec<VpnProfile>, CoreError> {
		self.profiles.list().await
	}

	async fn connect(&self, name: &str) -> Result<(), CoreError> {
		let out = self.exec.run("swanctl", &["--initiate", "--child", name]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn disconnect(&self, name: &str) -> Result<(), CoreError> {
		let out = self.exec.run("swanctl", &["--terminate", "--child", name]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn status(&self, name: &str) -> Result<ConnectionState, CoreError> {
		let out = self.exec.run("swanctl", &["--list-sas"]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		let s = out.stdout;
		if s.contains(name) {
			Ok(ConnectionState::Connected)
		} else {
			Ok(ConnectionState::Disconnected)
		}
	}
}


