use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError};

pub struct LinuxSplitRoutes {
	exec: Arc<dyn CommandExecutor>,
}

impl LinuxSplitRoutes {
	pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
		Self { exec }
	}

	pub async fn set_include(&self, iface: &str, cidrs: &[&str]) -> Result<(), CoreError> {
		for cidr in cidrs {
			let out = self.exec.run("ip", &["route", "replace", cidr, "dev", iface]).await?;
			if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		}
		Ok(())
	}

	pub async fn clear(&self, iface: &str, cidrs: &[&str]) -> Result<(), CoreError> {
		for cidr in cidrs {
			let _ = self.exec.run("ip", &["route", "del", cidr, "dev", iface]).await?;
		}
		Ok(())
	}
}
