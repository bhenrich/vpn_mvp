use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError};

pub struct MacDns {
	exec: Arc<dyn CommandExecutor>,
}

impl MacDns {
	pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
		Self { exec }
	}

	/// List all network services known to networksetup
	pub async fn list_services(&self) -> Result<Vec<String>, CoreError> {
		let out = self.exec.run("networksetup", &["-listallnetworkservices"]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		let mut services = Vec::new();
		for line in out.stdout.lines() {
			let name = line.trim();
			if name.is_empty() {
				continue;
			}
			// Skip header lines that some macOS versions include
			if name.starts_with("An asterisk") || name == "Network Service" {
				continue;
			}
			// Enabled services don't have leading asterisk; drop it if present
			let name = name.trim_start_matches('*').trim();
			if !name.is_empty() {
				services.push(name.to_string());
			}
		}
		Ok(services)
	}

	/// Set DNS servers for a given service (e.g., "Wi-Fi"). Use "Empty" to clear.
	pub async fn set_dns_for_service(&self, service: &str, servers: &[&str]) -> Result<(), CoreError> {
		let mut args: Vec<String> = vec!["-setdnsservers".into(), service.into()];
		if servers.is_empty() {
			args.push("Empty".into());
		} else {
			args.extend(servers.iter().map(|s| s.to_string()));
		}
		let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
		let out = self.exec.run("networksetup", &arg_refs).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}
}


