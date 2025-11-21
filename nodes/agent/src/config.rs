use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub grpc_addr: String,
    pub http_addr: String,
    pub wg_interface: String,
    pub wg_address: String,
    pub listen_port: u16,
    pub public_endpoint: String,
    pub region_id: Option<String>,
    pub region_code: Option<String>,
    pub region_city: Option<String>,
    pub egress_ips: Vec<String>,
}

impl AgentConfig {
    pub fn from_env() -> Result<Self> {
        let grpc_addr = std::env::var("DIRECTORY_GRPC_ADDR")
            .unwrap_or_else(|_| "http://127.0.0.1:50051".to_string());
        let http_addr = std::env::var("DIRECTORY_HTTP_ADDR")
            .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
        let wg_interface = std::env::var("WG_INTERFACE").unwrap_or_else(|_| "wg0".to_string());
        let wg_address = std::env::var("WG_ADDRESS").unwrap_or_else(|_| "10.66.0.1/24".to_string());
        let listen_port: u16 = std::env::var("WG_LISTEN_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(51820);
        let public_endpoint = std::env::var("PUBLIC_ENDPOINT")
            .map_err(|_| anyhow!("PUBLIC_ENDPOINT env var is required for node-agent"))?;

        let region_id = std::env::var("NODE_REGION_ID").ok();
        let region_code = std::env::var("NODE_REGION_CODE").ok();
        let region_city = std::env::var("NODE_REGION_CITY").ok();

        let egress_ips = std::env::var("NODE_EGRESS_IPS")
            .unwrap_or_default()
            .split(',')
            .filter_map(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect();

        if region_id.is_none() && (region_code.is_none() || region_city.is_none()) {
            return Err(anyhow!(
				"set NODE_REGION_ID or both NODE_REGION_CODE and NODE_REGION_CITY so the node can register"
			));
        }

        Ok(Self {
            grpc_addr,
            http_addr,
            wg_interface,
            wg_address,
            listen_port,
            public_endpoint,
            region_id,
            region_code,
            region_city,
            egress_ips,
        })
    }

    pub fn wg_internal_ip(&self) -> Result<String> {
        self.wg_address
            .split('/')
            .next()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                anyhow!("WG_ADDRESS must include an IPv4 with prefix, e.g. 10.66.0.1/24")
            })
    }
}
