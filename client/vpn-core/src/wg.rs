use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPair {
	pub private: [u8; 32],
	pub public: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowedIp {
	V4 { addr: Ipv4Addr, cidr: u8 },
	V6 { addr: Ipv6Addr, cidr: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
	pub public_key: [u8; 32],
	pub endpoint: Option<SocketAddr>,
	pub allowed_ips: Vec<AllowedIp>,
	pub persistent_keepalive_secs: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAddress {
	pub v4: Option<(Ipv4Addr, u8)>, // addr, prefix
	pub v6: Option<(Ipv6Addr, u8)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgDeviceConfig {
	pub name: String,
	pub mtu: Option<u32>,
	pub keypair: KeyPair,
	pub address: DeviceAddress,
	pub peers: Vec<Peer>,
}

#[async_trait::async_trait]
pub trait TunBackend: Send + Sync {
	/// Create or open a TUN interface with the provided name.
	async fn open(&self, name: &str) -> Result<(), CoreError>;
	/// Bring the TUN interface up.
	async fn up(&self) -> Result<(), CoreError>;
	/// Bring the TUN interface down.
	async fn down(&self) -> Result<(), CoreError>;
	/// Set interface MTU if supported.
	async fn set_mtu(&self, mtu: u32) -> Result<(), CoreError>;
	/// Assign IPv4/IPv6 addresses.
	async fn set_address(&self, address: &DeviceAddress) -> Result<(), CoreError>;
}

#[async_trait::async_trait]
pub trait SessionManager: Send + Sync {
	/// Start a WireGuard session: configures TUN and peers, and begins processing.
	async fn start(&self, cfg: &WgDeviceConfig) -> Result<(), CoreError>;
	/// Stop the running session and tear down state.
	async fn stop(&self) -> Result<(), CoreError>;
	/// Update peer configuration live.
	async fn update_peers(&self, peers: &[Peer]) -> Result<(), CoreError>;
}


