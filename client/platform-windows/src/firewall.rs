use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError};

pub struct WindowsFirewall {
	exec: Arc<dyn CommandExecutor>,
}

impl WindowsFirewall {
	pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
		Self { exec }
	}

	pub async fn apply_killswitch(&self, iface: &str, allow_ips_v4: &[&str], allow_ips_v6: &[&str]) -> Result<(), CoreError> {
		// Remove existing rules from prior runs
		let _ = self.exec.run("powershell", &["-NoProfile", "-NonInteractive", "-Command",
			"Get-NetFirewallRule -DisplayName 'VPNMvp-*' | Remove-NetFirewallRule"]).await?;

		// Allow outbound on the VPN interface
		let allow_iface = format!("New-NetFirewallRule -DisplayName 'VPNMvp-Allow-Tun' -Direction Outbound -InterfaceAlias '{}' -Action Allow", iface);
		let out = self.exec.run("powershell", &["-NoProfile","-NonInteractive","-Command",&allow_iface]).await?;
		if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }

		// Allow control plane IPs outbound (any interface) so auth can proceed
		for ip in allow_ips_v4 {
			let cmd = format!("New-NetFirewallRule -DisplayName 'VPNMvp-Allow-CPv4-{}' -Direction Outbound -RemoteAddress {} -Action Allow", ip, ip);
			let out = self.exec.run("powershell", &["-NoProfile","-NonInteractive","-Command",&cmd]).await?;
			if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		}
		for ip in allow_ips_v6 {
			let cmd = format!("New-NetFirewallRule -DisplayName 'VPNMvp-Allow-CPv6-{}' -Direction Outbound -RemoteAddress {} -Action Allow", ip, ip);
			let out = self.exec.run("powershell", &["-NoProfile","-NonInteractive","-Command",&cmd]).await?;
			if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		}

		// Finally, block all remaining outbound
		let block_all = "New-NetFirewallRule -DisplayName 'VPNMvp-Block-All' -Direction Outbound -Action Block";
		let out = self.exec.run("powershell", &["-NoProfile","-NonInteractive","-Command",block_all]).await?;
		if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		Ok(())
	}

	pub async fn remove_killswitch(&self) -> Result<(), CoreError> {
		let out = self.exec.run("powershell", &["-NoProfile","-NonInteractive","-Command",
			"Get-NetFirewallRule -DisplayName 'VPNMvp-*' | Remove-NetFirewallRule"]).await?;
		if out.status != 0 {
			// tolerate missing rules
		}
		Ok(())
	}

	/// Allow a specific program to bypass the kill switch (outbound allow).
	pub async fn allow_program(&self, program_path: &str) -> Result<(), CoreError> {
		let cmd = format!("New-NetFirewallRule -DisplayName 'VPNMvp-App-{}' -Direction Outbound -Program '{}' -Action Allow",
			program_path.replace('\\', "/"),
			program_path.replace('\'', "''"));
		let out = self.exec.run("powershell", &["-NoProfile","-NonInteractive","-Command",&cmd]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	/// Remove all app-allow rules created by allow_program.
	pub async fn clear_app_rules(&self) -> Result<(), CoreError> {
		let out = self.exec.run("powershell", &["-NoProfile","-NonInteractive","-Command",
			"Get-NetFirewallRule -DisplayName 'VPNMvp-App-*' | Remove-NetFirewallRule"]).await?;
		if out.status != 0 {
			// tolerate missing
		}
		Ok(())
	}
}


