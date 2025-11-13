use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError};

pub struct WindowsDns {
	exec: Arc<dyn CommandExecutor>,
}

impl WindowsDns {
	pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
		Self { exec }
	}

	pub async fn set_for_interface(&self, iface: &str, servers: &[&str]) -> Result<(), CoreError> {
		let mut script = String::new();
		let servers_list = if servers.is_empty() {
			"".to_string()
		} else {
			format!("@({})", servers.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(","))
		};
		if servers.is_empty() {
			script.push_str(&format!("Set-DnsClientServerAddress -InterfaceAlias \"{}\" -ResetServerAddresses", iface));
		} else {
			script.push_str(&format!(
				"Set-DnsClientServerAddress -InterfaceAlias \"{}\" -ServerAddresses {}",
				iface, servers_list
			));
		}
		let args = ["-NoProfile", "-NonInteractive", "-Command", &script];
		let out = self.exec.run("powershell", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}
}


