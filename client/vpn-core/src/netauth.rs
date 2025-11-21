use serde::{Deserialize, Serialize};

use crate::CoreError;

#[derive(Clone)]
pub struct AuthApiClient {
	base_url: String,
	http: reqwest::Client,
}

impl AuthApiClient {
	pub fn new_from_env() -> Result<Self, CoreError> {
		let base_url = std::env::var("AUTH_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
		Ok(Self::new(base_url))
	}

	pub fn new(base_url: String) -> Self {
		Self {
			base_url,
			http: reqwest::Client::new(),
		}
	}

	pub async fn login(&self, email: &str, password: &str) -> Result<LoginResponse, CoreError> {
		let url = format!("{}/api/v1/auth/login", self.base_url);
		let payload = LoginRequest {
			email: email.to_string(),
			password: password.to_string(),
		};
		let resp = self
			.http
			.post(url)
			.json(&payload)
			.send()
			.await
			.map_err(|e| CoreError::Other(format!("auth login failed: {e}")))?;
		if !resp.status().is_success() {
			return Err(CoreError::Other(format!("auth login http {}", resp.status())));
		}
		let data = resp
			.json::<LoginResponse>()
			.await
			.map_err(|e| CoreError::Other(format!("auth login parse: {e}")))?;
		Ok(data)
	}

	pub async fn fetch_profile(&self, access_token: &str) -> Result<ServerProfile, CoreError> {
		let url = format!("{}/api/v1/vpn/profile", self.base_url);
		let resp = self
			.http
			.get(url)
			.bearer_auth(access_token)
			.send()
			.await
			.map_err(|e| CoreError::Other(format!("fetch profile failed: {e}")))?;
		if !resp.status().is_success() {
			return Err(CoreError::Other(format!("fetch profile http {}", resp.status())));
		}
		let text = resp.text().await.map_err(|e| CoreError::Other(format!("read profile: {e}")))?;
		// Try JSON first; if not JSON, parse as OpenVPN config.
		match serde_json::from_str::<ServerProfile>(&text) {
			Ok(p) => Ok(p),
			Err(_) => {
				// Parse OpenVPN config format
				let server = Self::parse_openvpn_remote(&text);
				Ok(ServerProfile {
					name: Some("default".to_string()),
					server,
					remote_id: None,
					username: None,
					password: None,
					dns: Vec::new(),
					split_tunnel: None,
					raw: Some(text),
				})
			},
		}
	}

	fn parse_openvpn_remote(config: &str) -> Option<String> {
		for line in config.lines() {
			let line = line.trim();
			if line.starts_with("remote ") {
				let parts: Vec<&str> = line.split_whitespace().collect();
				if parts.len() >= 2 {
					let host = parts[1];
					let port = if parts.len() >= 3 { parts[2] } else { "1194" };
					// For Windows IKEv2, we typically just need the host
					// But if port is not 1194, include it
					if port == "1194" {
						return Some(host.to_string());
					} else {
						return Some(format!("{}:{}", host, port));
					}
				}
			}
		}
		None
	}
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
	email: String,
	password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
	#[serde(rename = "accessToken")]
	pub access_token: String,
	#[serde(rename = "refreshToken")]
	pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerProfile {
	pub name: Option<String>,
	pub server: Option<String>,
	pub remote_id: Option<String>,
	pub username: Option<String>,
	pub password: Option<String>,
	#[serde(default)]
	pub dns: Vec<String>,
	pub split_tunnel: Option<bool>,
	#[serde(skip)]
	pub raw: Option<String>,
}

impl ServerProfile {
	pub fn infer_profile_name(&self) -> String {
		self.name
			.clone()
			.or_else(|| self.server.clone())
			.unwrap_or_else(|| "default".to_string())
	}
}


