use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError};

/// Manage DNS settings via systemd-resolved for a specific interface.
pub struct LinuxResolvedDns {
	exec: Arc<dyn CommandExecutor>,
}

impl LinuxResolvedDns {
	pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
		Self { exec }
	}

	pub async fn set_dns(&self, iface: &str, servers: &[&str]) -> Result<(), CoreError> {
		if servers.is_empty() {
			return Ok(());
		}
		let mut args = vec!["dns", iface];
		for s in servers {
			args.push(s);
		}
		let out = self.exec.run("resolvectl", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		// Set "routing domains" to only use DNS on this link for all queries, to prevent leaks.
		// '~.' marks a routing-only domain.
		let out = self.exec.run("resolvectl", &["domain", iface, "~."]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	pub async fn revert(&self, iface: &str) -> Result<(), CoreError> {
		let out = self.exec.run("resolvectl", &["revert", iface]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}
}


