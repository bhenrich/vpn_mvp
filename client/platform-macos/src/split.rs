use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError};

pub struct MacSplitRoutes {
	exec: Arc<dyn CommandExecutor>,
}

impl MacSplitRoutes {
	pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
		Self { exec }
	}

	pub async fn set_include(&self, iface: &str, cidrs: &[&str]) -> Result<(), CoreError> {
		for cidr in cidrs {
			// route -n add -net <cidr> -interface <iface>
			let out = self
				.exec
				.run("route", &["-n", "add", "-net", cidr, "-interface", iface])
				.await?;
			if out.status != 0 {
				return Err(CoreError::CommandFailed(out.stderr));
			}
		}
		Ok(())
	}

	pub async fn clear(&self, iface: &str, cidrs: &[&str]) -> Result<(), CoreError> {
		for cidr in cidrs {
			let _ = self.exec.run("route", &["-n", "delete", "-net", cidr, "-interface", iface]).await?;
		}
		Ok(())
	}
}


