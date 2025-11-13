use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError};

pub struct WindowsSplitRoutes {
	exec: Arc<dyn CommandExecutor>,
}

impl WindowsSplitRoutes {
	pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
		Self { exec }
	}

	pub async fn set_include(&self, iface: &str, cidrs: &[&str]) -> Result<(), CoreError> {
		for cidr in cidrs {
			// Prefer PowerShell New-NetRoute (ActiveStore). NextHop may be omitted for on-link routes.
			let cmd = format!(
				"New-NetRoute -DestinationPrefix {} -InterfaceAlias \"{}\" -PolicyStore ActiveStore -ErrorAction Stop",
				cidr, iface
			);
			let out = self
				.exec
				.run("powershell", &["-NoProfile", "-NonInteractive", "-Command", &cmd])
				.await?;
			if out.status != 0 {
				return Err(CoreError::CommandFailed(out.stderr));
			}
		}
		Ok(())
	}

	pub async fn clear(&self, iface: &str, cidrs: &[&str]) -> Result<(), CoreError> {
		for cidr in cidrs {
			let cmd = format!(
				"Get-NetRoute -DestinationPrefix {} -InterfaceAlias \"{}\" | Remove-NetRoute -Confirm:$false",
				cidr, iface
			);
			let _ = self
				.exec
				.run("powershell", &["-NoProfile", "-NonInteractive", "-Command", &cmd])
				.await?;
		}
		Ok(())
	}
}


