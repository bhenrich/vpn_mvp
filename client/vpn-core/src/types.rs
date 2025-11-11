use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMethod {
	/// EAP-MSCHAPv2 (username/password). Username may be stored; password is kept in OS keychain.
	EapMsChapV2 {
		username: Option<String>,
	},
	/// Placeholder for future EAP-TLS support.
	EapTls {
		identity: Option<String>,
		cert_ref: Option<String>,
	},
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionState {
	Disconnected,
	Connecting,
	Connected,
	Disconnecting,
	Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DnsSettings {
	pub servers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SplitTunnel {
	pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VpnProfile {
	pub name: String,
	pub server: String,
	pub remote_id: Option<String>,
	pub auth: AuthMethod,
	pub split_tunnel: SplitTunnel,
	pub dns: DnsSettings,
}


