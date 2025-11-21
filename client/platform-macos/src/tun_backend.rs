use std::sync::Arc;

use tun::{AsyncDevice, Configuration};
use tokio::sync::Mutex;
use vpn_core::{CoreError, DeviceAddress, TunBackend};
use vpn_core::CommandExecutor;

pub struct MacOsUtunBackend {
	exec: Arc<dyn CommandExecutor>,
	name: String,
	dev: Mutex<Option<AsyncDevice>>,
}

impl MacOsUtunBackend {
	pub fn new(exec: Arc<dyn CommandExecutor>, name: String) -> Self {
		Self { exec, name, dev: Mutex::new(None) }
	}

	fn ifname(&self) -> &str { &self.name }
}

#[async_trait::async_trait]
impl TunBackend for MacOsUtunBackend {
	async fn open(&self, name: &str) -> Result<(), CoreError> {
		if name != self.name { return Err(CoreError::InvalidInput("iface name mismatch".into())); }
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
		let name = self.ifname();
		let out = self.exec.run("ifconfig", &[name, "up"]).await?;
		if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		Ok(())
	}

	async fn down(&self) -> Result<(), CoreError> {
		let name = self.ifname();
		let out = self.exec.run("ifconfig", &[name, "down"]).await?;
		if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		Ok(())
	}

	async fn set_mtu(&self, mtu: u32) -> Result<(), CoreError> {
		let name = self.ifname();
		let mtu_s = mtu.to_string();
		let out = self.exec.run("ifconfig", &[name, "mtu", &mtu_s]).await?;
		if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		Ok(())
	}

	async fn set_address(&self, address: &DeviceAddress) -> Result<(), CoreError> {
		let name = self.ifname();
		if let Some((addr, prefix)) = address.v4 {
			// Use alias assignment with prefix; macOS derives netmask from prefix
			let cidr = format!("{}/{}", addr, prefix);
			let out = self.exec.run("ifconfig", &[name, "inet", &cidr, "alias"]).await?;
			if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		}
		if let Some((addr, prefix)) = address.v6 {
			let cidr = format!("{}/{}", addr, prefix);
			let out = self.exec.run("ifconfig", &[name, "inet6", &cidr, "add"]).await?;
			if out.status != 0 { return Err(CoreError::CommandFailed(out.stderr)); }
		}
		Ok(())
	}
}
