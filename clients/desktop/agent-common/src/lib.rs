use axum::{
    response::{IntoResponse, Response},
    Json,
};
use base64::Engine as _;
use desktop_core::SessionPolicy;
use serde::{Deserialize, Serialize};
use vpn_core::wg::{AllowedIp, DeviceAddress, KeyPair, Peer, WgDeviceConfig};
use vpn_core::CoreError;
use x25519_dalek::StaticSecret;

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: &'static str,
    pub interface: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BasicResponse {
    pub status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct ConnectRequest {
    pub interface: String,
    pub private_key: String,
    pub address_v4: String,
    pub address_v6: Option<String>,
    pub peer: PeerRequest,
    pub dns_servers: Vec<String>,
    pub routes_v4: Vec<String>,
    pub routes_v6: Vec<String>,
    pub kill_switch_allow_v4: Vec<String>,
    pub kill_switch_allow_v6: Vec<String>,
    pub kill_switch_allow_uids: Vec<u32>,
    pub mtu: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct PeerRequest {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive_secs: Option<u16>,
}

impl ConnectRequest {
    pub fn into_session(self) -> Result<(WgDeviceConfig, SessionPolicy), AgentError> {
        let keypair = Self::decode_private_key(&self.private_key)?;
        let address = DeviceAddress {
            v4: Some(Self::parse_ipv4_cidr(&self.address_v4)?),
            v6: match &self.address_v6 {
                Some(s) if !s.is_empty() => Some(Self::parse_ipv6_cidr(s)?),
                _ => None,
            },
        };

        let peer = self.peer.to_peer()?;
        let config = WgDeviceConfig {
            name: self.interface.clone(),
            mtu: self.mtu,
            keypair,
            address,
            peers: vec![peer],
        };

        let policy = SessionPolicy {
            interface: self.interface.clone(),
            dns_servers: self.dns_servers,
            routes_v4: self.routes_v4,
            routes_v6: self.routes_v6,
            kill_switch_allow_v4: self.kill_switch_allow_v4,
            kill_switch_allow_v6: self.kill_switch_allow_v6,
            kill_switch_allow_uids: self.kill_switch_allow_uids,
            mtu: self.mtu,
        };

        Ok((config, policy))
    }

    fn decode_private_key(b64: &str) -> Result<KeyPair, AgentError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64.trim())
            .map_err(|_| AgentError::bad_request("invalid private key"))?;
        if bytes.len() != 32 {
            return Err(AgentError::bad_request("private key must be 32 bytes"));
        }
        let mut priv_bytes = [0u8; 32];
        priv_bytes.copy_from_slice(&bytes);
        let secret = StaticSecret::from(priv_bytes);
        let public = x25519_dalek::PublicKey::from(&secret);
        let mut pub_bytes = [0u8; 32];
        pub_bytes.copy_from_slice(public.as_bytes());
        Ok(KeyPair {
            private: priv_bytes,
            public: pub_bytes,
        })
    }

    fn parse_ipv4_cidr(value: &str) -> Result<(std::net::Ipv4Addr, u8), AgentError> {
        let (addr, prefix) = value
            .split_once('/')
            .ok_or_else(|| AgentError::bad_request("invalid IPv4 CIDR"))?;
        let ip = addr
            .parse()
            .map_err(|_| AgentError::bad_request("invalid IPv4 address"))?;
        let cidr: u8 = prefix
            .parse()
            .map_err(|_| AgentError::bad_request("invalid IPv4 prefix"))?;
        if cidr > 32 {
            return Err(AgentError::bad_request("IPv4 prefix out of range"));
        }
        Ok((ip, cidr))
    }

    fn parse_ipv6_cidr(value: &str) -> Result<(std::net::Ipv6Addr, u8), AgentError> {
        let (addr, prefix) = value
            .split_once('/')
            .ok_or_else(|| AgentError::bad_request("invalid IPv6 CIDR"))?;
        let ip = addr
            .parse()
            .map_err(|_| AgentError::bad_request("invalid IPv6 address"))?;
        let cidr: u8 = prefix
            .parse()
            .map_err(|_| AgentError::bad_request("invalid IPv6 prefix"))?;
        if cidr > 128 {
            return Err(AgentError::bad_request("IPv6 prefix out of range"));
        }
        Ok((ip, cidr))
    }
}

impl PeerRequest {
    fn to_peer(&self) -> Result<Peer, AgentError> {
        let public_key = decode_key(&self.public_key)?;
        let allowed_ips = self
            .allowed_ips
            .iter()
            .map(|cidr| parse_allowed_ip(cidr))
            .collect::<Result<Vec<_>, _>>()?;
        if allowed_ips.is_empty() {
            return Err(AgentError::bad_request("peer allowed_ips cannot be empty"));
        }
        let endpoint = match &self.endpoint {
            Some(ep) if !ep.is_empty() => Some(
                ep.parse()
                    .map_err(|_| AgentError::bad_request("invalid peer endpoint"))?,
            ),
            _ => None,
        };
        Ok(Peer {
            public_key,
            endpoint,
            allowed_ips,
            persistent_keepalive_secs: self.persistent_keepalive_secs,
        })
    }
}

fn decode_key(b64: &str) -> Result<[u8; 32], AgentError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .map_err(|_| AgentError::bad_request("invalid public key"))?;
    if bytes.len() != 32 {
        return Err(AgentError::bad_request("public key must be 32 bytes"));
    }
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&bytes);
    Ok(buf)
}

fn parse_allowed_ip(value: &str) -> Result<AllowedIp, AgentError> {
    let (addr, prefix) = value
        .split_once('/')
        .ok_or_else(|| AgentError::bad_request("invalid allowed IP"))?;
    let cidr: u8 = prefix
        .parse()
        .map_err(|_| AgentError::bad_request("invalid CIDR prefix"))?;
    let ip: std::net::IpAddr = addr
        .parse()
        .map_err(|_| AgentError::bad_request("invalid CIDR address"))?;
    match ip {
        std::net::IpAddr::V4(v4) => {
            if cidr > 32 {
                return Err(AgentError::bad_request("IPv4 CIDR out of range"));
            }
            Ok(AllowedIp::V4 { addr: v4, cidr })
        }
        std::net::IpAddr::V6(v6) => {
            if cidr > 128 {
                return Err(AgentError::bad_request("IPv6 CIDR out of range"));
            }
            Ok(AllowedIp::V6 { addr: v6, cidr })
        }
    }
}

#[derive(Debug)]
pub enum AgentError {
    Core(CoreError),
    BadRequest(String),
    Conflict(String),
    NotImplemented(String),
}

impl AgentError {
    pub fn bad_request(msg: &str) -> Self {
        Self::BadRequest(msg.to_string())
    }
}

impl From<CoreError> for AgentError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::InvalidInput(msg) => AgentError::BadRequest(msg),
            CoreError::AlreadyExists(msg) => AgentError::Conflict(msg),
            CoreError::UnsupportedPlatform => {
                AgentError::NotImplemented("platform not yet supported".into())
            }
            other => AgentError::Core(other),
        }
    }
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl IntoResponse for AgentError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AgentError::BadRequest(msg) => (axum::http::StatusCode::BAD_REQUEST, msg.clone()),
            AgentError::Conflict(msg) => (axum::http::StatusCode::CONFLICT, msg.clone()),
            AgentError::NotImplemented(msg) => {
                (axum::http::StatusCode::NOT_IMPLEMENTED, msg.clone())
            }
            AgentError::Core(err) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                err.to_string(),
            ),
        };
        (status, Json(ErrorResponse { error: msg })).into_response()
    }
}
