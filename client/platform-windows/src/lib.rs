//! Windows-specific adapter crate (placeholder scaffolding).
//! Real implementation will invoke PowerShell and RAS for IKEv2 connections.

use std::sync::Arc;

use vpn_core::{
		AuthMethod, CommandExecutor, ConnectionState, CoreError, CredentialStore, ProfileStore, VpnAdapter,
	VpnProfile,
};

pub struct WindowsAdapter {
	exec: Arc<dyn CommandExecutor>,
	creds: Arc<dyn CredentialStore>,
	profiles: Arc<dyn ProfileStore>,
}

impl WindowsAdapter {
	pub fn new(exec: Arc<dyn CommandExecutor>, creds: Arc<dyn CredentialStore>, profiles: Arc<dyn ProfileStore>) -> Self {
		Self { exec, creds, profiles }
	}
}

pub mod tun_backend;
pub mod dns;
pub mod firewall;
pub mod split;

#[async_trait::async_trait]
impl VpnAdapter for WindowsAdapter {
	async fn init(&self) -> Result<(), CoreError> {
		Ok(())
	}

	async fn add_profile(&self, profile: VpnProfile) -> Result<(), CoreError> {
		let mut args: Vec<String> = Vec::new();
		args.push("-NoProfile".into());
		args.push("-NonInteractive".into());
		let script = build_add_vpn_ps(&profile);
		args.push("-Command".into());
		args.push(script);
		let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
		let out = self.exec.run("powershell", &arg_refs).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		// Persist profile to our store for username lookup later
		self.profiles.save(&profile).await?;
		Ok(())
	}

	async fn remove_profile(&self, name: &str) -> Result<(), CoreError> {
		let script = format!(
			"Remove-VpnConnection -Name '{}' -Force -PassThru | Out-Null",
			escape_ps(name)
		);
		let args = ["-NoProfile", "-NonInteractive", "-Command", &script];
		let out = self.exec.run("powershell", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		let _ = self.profiles.delete(name).await;
		Ok(())
	}

	async fn list_profiles(&self) -> Result<Vec<VpnProfile>, CoreError> {
		// Return stored profiles (best-effort view)
		self.profiles.list().await
	}

	async fn connect(&self, name: &str) -> Result<(), CoreError> {
		println!("[VPN] Attempting to connect to profile: {}", name);
		let profile = self.profiles.load(name).await?;
		println!("[VPN] Loaded profile - Server: {}", profile.server);
		let (username, password) = match &profile.auth {
			AuthMethod::EapMsChapV2 { username } => {
				let user = username.clone().ok_or_else(|| CoreError::InvalidInput("username missing".into()))?;
				let pass = self
					.creds
					.get_password(&profile.name, &user)
					.await?
					.ok_or_else(|| CoreError::InvalidInput("password missing in keyring".into()))?;
				(user, pass)
			}
			_ => {
				return Err(CoreError::InvalidInput(
					"connect requires EAP-MSCHAPv2 credentials for MVP".into(),
				))
			}
		};
		let args = ["-NoProfile", "-NonInteractive", "-Command", "exit (rasdial)"];
		let _ = self.exec.run("powershell", &args).await?; // ensure rasdial exists

		println!("[VPN] Connecting via rasdial to: {} (user: {})", name, username);
		let args = ["rasdial", name, &username, &password];
		let out = self.exec.run(args[0], &args[1..]).await?;
		if out.status != 0 {
			eprintln!("[VPN] Connection failed - Status: {}, Error: {}", out.status, out.stderr);
			return Err(CoreError::CommandFailed(out.stderr));
		}
		println!("[VPN] ✓ Connection established successfully to: {} ({}:{})", name, profile.server, profile.server.split(':').nth(1).unwrap_or(""));
		eprintln!("[VPN] Connection output: {}", out.stdout);
		Ok(())
	}

	async fn disconnect(&self, name: &str) -> Result<(), CoreError> {
		println!("[VPN] Disconnecting from profile: {}", name);
		let args = ["rasdial", name, "/disconnect"];
		let out = self.exec.run(args[0], &args[1..]).await?;
		if out.status != 0 {
			eprintln!("[VPN] Disconnect failed - Status: {}, Error: {}", out.status, out.stderr);
			return Err(CoreError::CommandFailed(out.stderr));
		}
		println!("[VPN] ✓ Disconnected from: {}", name);
		Ok(())
	}

	async fn status(&self, name: &str) -> Result<ConnectionState, CoreError> {
		// Check connection status via rasdial
		let args = ["rasdial", name];
		let out = self.exec.run(args[0], &args[1..]).await?;
		
		// rasdial returns 0 if connected, non-zero if disconnected/error
		if out.status == 0 && out.stdout.contains("Connected to") {
			// Parse connection info and log packet stats
			let _ = WindowsAdapter::log_packet_stats(&self.exec, name).await;
			return Ok(ConnectionState::Connected);
		}
		Ok(ConnectionState::Disconnected)
	}
}

impl WindowsAdapter {
	async fn log_packet_stats(exec: &Arc<dyn CommandExecutor>, name: &str) -> Result<(), CoreError> {
		// Get VPN adapter interface stats using PowerShell
		let script = format!(
			r#"Get-NetAdapter | Where-Object {{ $_.InterfaceDescription -like '*{}*' -or $_.Name -like '*{}*' }} | Get-NetAdapterStatistics | Select-Object -Property Name, BytesReceived, BytesSent, PacketsReceived, PacketsSent | ConvertTo-Json"#,
			name, name
		);
		let args = ["-NoProfile", "-NonInteractive", "-Command", &script];
		let out = exec.run("powershell", &args).await?;
		
		if out.status == 0 && !out.stdout.trim().is_empty() {
			// Try to parse and log stats
			let stats = out.stdout.trim();
			if !stats.contains("null") && stats.len() > 10 {
				println!("[VPN] Packet stats for {}: {}", name, stats);
			}
		}
		
		// Also check rasdial status for connection info
		let args = ["rasdial", name];
		let out = exec.run(args[0], &args[1..]).await?;
		if out.status == 0 && out.stdout.contains("Connected to") {
			// Extract connection details from rasdial output
			let lines: Vec<&str> = out.stdout.lines().collect();
			for line in lines {
				if line.contains("Bytes") || line.contains("Compression") || line.contains("Errors") {
					println!("[VPN] {} - {}", name, line.trim());
				}
			}
		}
		Ok(())
	}
}

fn ps_bool(b: bool) -> &'static str {
	if b { "$true" } else { "$false" }
}

fn escape_ps(s: &str) -> String {
	s.replace('\'', "''")
}

fn build_add_vpn_ps(p: &VpnProfile) -> String {
	let mut parts: Vec<String> = Vec::new();
	parts.push(format!("Add-VpnConnection -Name '{}' -ServerAddress '{}'", escape_ps(&p.name), escape_ps(&p.server)));
	parts.push("-TunnelType IKEv2".into());
	parts.push("-AuthenticationMethod EAP".into());
	if let Some(remote_id) = &p.remote_id {
		parts.push(format!("-DnsSuffix '{}' -RememberCredential:$false", escape_ps(remote_id)));
	}
	match &p.auth {
		AuthMethod::EapMsChapV2 { .. } => {
			// Default EAP config typically includes MSCHAPv2; specific EAP XML can be set later if needed.
		}
		_ => {}
	}
	if p.split_tunnel.enabled {
		parts.push(format!("-SplitTunneling {}", ps_bool(true)));
	}
	if !p.dns.servers.is_empty() {
		let servers: Vec<String> = p.dns.servers.iter().map(|s| format!("'{}'", escape_ps(s))).collect();
		parts.push(format!("-DnsServerAddress @({})", servers.join(",")));
	}
	parts.push("-Force -PassThru | Out-Null".into());
	format!("{}", parts.join(" "))
}


