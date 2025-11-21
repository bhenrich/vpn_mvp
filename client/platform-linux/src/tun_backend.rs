use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError, DeviceAddress, TunBackend};
use tun::{AsyncDevice, Configuration};

use tokio::sync::Mutex;

pub struct LinuxIpTunBackend {
	exec: Arc<dyn CommandExecutor>,
	iface: String,
	dev: Mutex<Option<AsyncDevice>>,
}

impl LinuxIpTunBackend {
	pub fn new(exec: Arc<dyn CommandExecutor>, iface: String) -> Self {
		Self { exec, iface, dev: Mutex::new(None) }
	}

	fn ifname(&self) -> Result<&str, CoreError> {
		Ok(&self.iface)
	}
}

#[async_trait::async_trait]
impl TunBackend for LinuxIpTunBackend {
	async fn open(&self, name: &str) -> Result<(), CoreError> {
		if name != self.iface {
			return Err(CoreError::InvalidInput("iface name mismatch".into()));
		}
		// Create TUN device using tun crate (ensure non-Send config isn't held across await)
		let dev = {
			let mut cfg = Configuration::default();
			cfg.name(name).layer(tun::Layer::L3);
			tun::create_as_async(&cfg).map_err(|e| CoreError::Other(format!("tun create failed: {e}")))?
		};
		{
			let mut guard = self.dev.lock().await;
			*guard = Some(dev);
		}
		Ok(())
	}

	async fn up(&self) -> Result<(), CoreError> {
		let name = self.ifname()?;
		let out = self.exec.run("ip", &["link", "set", "dev", name, "up"]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn down(&self) -> Result<(), CoreError> {
		let name = self.ifname()?;
		let out = self
			.exec
			.run("ip", &["link", "set", "dev", name, "down"])
			.await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn set_mtu(&self, mtu: u32) -> Result<(), CoreError> {
		let name = self.ifname()?;
		let mtu = mtu.to_string();
		let out = self
			.exec
			.run("ip", &["link", "set", "dev", name, "mtu", &mtu])
			.await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	async fn set_address(&self, address: &DeviceAddress) -> Result<(), CoreError> {
		let name = self.ifname()?;
		if let Some((addr, prefix)) = address.v4 {
			let cidr = format!("{}/{}", addr, prefix);
			let out = self
				.exec
				.run("ip", &["addr", "replace", &cidr, "dev", name])
				.await?;
			if out.status != 0 {
				return Err(CoreError::CommandFailed(out.stderr));
			}
		}
		if let Some((addr, prefix)) = address.v6 {
			let cidr = format!("{}/{}", addr, prefix);
			let out = self
				.exec
				.run("ip", &["-6", "addr", "replace", &cidr, "dev", name])
				.await?;
			if out.status != 0 {
				return Err(CoreError::CommandFailed(out.stderr));
			}
		}
		Ok(())
	}
}


