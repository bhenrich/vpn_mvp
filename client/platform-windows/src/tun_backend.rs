use std::net::Ipv4Addr;
use std::sync::Arc;

use vpn_core::{CoreError, DeviceAddress, TunBackend};

use vpn_core::CommandExecutor;

pub struct WindowsWintunBackend {
	exec: Arc<dyn CommandExecutor>,
	iface: String,
}

impl WindowsWintunBackend {
	pub fn new(exec: Arc<dyn CommandExecutor>, iface: String) -> Self {
		Self { exec, iface }
	}

	fn ifname(&self) -> &str {
		&self.iface
	}
}

#[async_trait::async_trait]
impl TunBackend for WindowsWintunBackend {
	async fn open(&self, name: &str) -> Result<(), CoreError> {
		if name != self.iface {
			return Err(CoreError::InvalidInput("iface name mismatch".into()));
		}
		// Best-effort: ensure the interface exists using netsh; if not found, return error.
		let args = ["interface", "show", "interface", &format!("name=\"{}\"", name)];
		let out = self.exec.run("netsh", &args).await?;
		if out.status != 0 {
			return Err(CoreError::Other(format!("interface not found or inaccessible: {}", name)));
		}
		Ok(())
	}

	async fn up(&self) -> Result<(), CoreError> {
		let name = self.ifname();
		let args = ["interface", "set", "interface", &format!("name=\"{}\"", name), "admin=enabled"];
		let out = self.exec.run("netsh", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn down(&self) -> Result<(), CoreError> {
		let name = self.ifname();
		let args = ["interface", "set", "interface", &format!("name=\"{}\"", name), "admin=disabled"];
		let out = self.exec.run("netsh", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn set_mtu(&self, mtu: u32) -> Result<(), CoreError> {
		let name = self.ifname();
		let args = [
			"interface",
			"ipv4",
			"set",
			"subinterface",
			&format!("\"{}\"", name),
			&format!("mtu={}", mtu),
			"store=active",
		];
		let out = self.exec.run("netsh", &args).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn set_address(&self, address: &DeviceAddress) -> Result<(), CoreError> {
		let name = self.ifname();
		if let Some((addr, prefix)) = address.v4 {
			let mask = ipv4_mask_from_prefix(prefix).ok_or_else(|| CoreError::InvalidInput("invalid v4 prefix".into()))?;
			let args = [
				"interface",
				"ip",
				"add",
				"address",
				&format!("\"{}\"", name),
				&addr.to_string(),
				&mask.to_string(),
			];
			let out = self.exec.run("netsh", &args).await?;
			if out.status != 0 {
				return Err(CoreError::CommandFailed(out.stderr));
			}
		}
		if let Some((addr, prefix)) = address.v6 {
			let args = [
				"interface",
				"ipv6",
				"add",
				"address",
				&format!("\"{}\"", name),
				&format!("{}/{}", addr, prefix),
			];
			let out = self.exec.run("netsh", &args).await?;
			if out.status != 0 {
				return Err(CoreError::CommandFailed(out.stderr));
			}
		}
		Ok(())
	}
}

fn ipv4_mask_from_prefix(prefix: u8) -> Option<Ipv4Addr> {
	if prefix > 32 {
		return None;
	}
	let mask: u32 = if prefix == 0 { 0 } else { (!0u32) << (32 - prefix) };
	let octets = [
		((mask >> 24) & 0xff) as u8,
		((mask >> 16) & 0xff) as u8,
		((mask >> 8) & 0xff) as u8,
		(mask & 0xff) as u8,
	];
	Some(Ipv4Addr::from(octets))
}


