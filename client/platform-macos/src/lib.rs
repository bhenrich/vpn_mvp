//! macOS-specific adapter crate (placeholder scaffolding).
//! Real implementation will manage profiles and `scutil --nc` for IKEv2 connections.

use std::sync::Arc;

use vpn_core::{CommandExecutor, ConnectionState, CoreError, CredentialStore, ProfileStore, VpnAdapter, VpnProfile};

pub struct MacosAdapter {
	exec: Arc<dyn CommandExecutor>,
	creds: Arc<dyn CredentialStore>,
	profiles: Arc<dyn ProfileStore>,
}

impl MacosAdapter {
	pub fn new(exec: Arc<dyn CommandExecutor>, creds: Arc<dyn CredentialStore>, profiles: Arc<dyn ProfileStore>) -> Self {
		Self { exec, creds, profiles }
	}
}

#[async_trait::async_trait]
impl VpnAdapter for MacosAdapter {
	async fn init(&self) -> Result<(), CoreError> {
		Ok(())
	}

	async fn add_profile(&self, profile: VpnProfile) -> Result<(), CoreError> {
		// MVP: persist profile only; provisioning via .mobileconfig to be added later
		self.profiles.save(&profile).await
	}

	async fn remove_profile(&self, name: &str) -> Result<(), CoreError> {
		// Attempt removal from store; system profile removal will be handled when provisioning is implemented
		self.profiles.delete(name).await
	}

	async fn list_profiles(&self) -> Result<Vec<VpnProfile>, CoreError> {
		self.profiles.list().await
	}

	async fn connect(&self, name: &str) -> Result<(), CoreError> {
		let args = ["--nc", "start", name];
		let out = self.exec.run("scutil", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn disconnect(&self, name: &str) -> Result<(), CoreError> {
		let args = ["--nc", "stop", name];
		let out = self.exec.run("scutil", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn status(&self, name: &str) -> Result<ConnectionState, CoreError> {
		let args = ["--nc", "show", name];
		let out = self.exec.run("scutil", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		let s = out.stdout;
		let state = if s.contains("Connected") {
			ConnectionState::Connected
		} else if s.contains("Connecting") {
			ConnectionState::Connecting
		} else if s.contains("Disconnected") {
			ConnectionState::Disconnected
		} else {
			ConnectionState::Unknown
		};
		Ok(state)
	}
}


