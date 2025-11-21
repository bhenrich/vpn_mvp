use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct DirectoryHttpClient {
    base: Url,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
pub struct RegionRecord {
    pub id: String,
    pub country_code: String,
    pub city: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct NodeRegisterRequest {
    pub public_key: String,
    pub region_id: Option<String>,
    pub internal_wg_ip: Option<String>,
    pub egress_ips: Option<Vec<String>>,
    pub public_endpoint: Option<String>,
    pub listen_port: Option<u16>,
    pub capabilities: Option<serde_json::Value>,
    pub agent_version: Option<String>,
    pub wg_rs_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NodeHeartbeatRequest {
    pub public_key: String,
    pub status: Option<String>,
    pub agent_version: Option<String>,
    pub wg_rs_version: Option<String>,
}

impl DirectoryHttpClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let base = Url::parse(base_url)
            .with_context(|| format!("invalid DIRECTORY_HTTP_ADDR: {base_url}"))?;
        let http = reqwest::Client::builder()
            .user_agent("vpn-node-agent")
            .build()?;
        Ok(Self { base, http })
    }

    fn url_for(&self, path: &str) -> Result<Url> {
        self.base
            .join(path)
            .with_context(|| format!("failed to build directory-api URL for {path}"))
    }

    pub async fn list_regions(&self) -> Result<Vec<RegionRecord>> {
        let url = self.url_for("/regions")?;
        let resp = self.http.get(url).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("list_regions failed: {status} {body}"));
        }
        let regions: Vec<RegionRecord> = resp.json().await?;
        Ok(regions)
    }

    pub async fn resolve_region_id(&self, code: &str, city: &str) -> Result<String> {
        let target_code = code.trim().to_uppercase();
        let target_city = city.trim().to_lowercase();
        let regions = self.list_regions().await?;
        for region in regions {
            if region
                .country_code
                .trim()
                .eq_ignore_ascii_case(&target_code)
                && region.city.trim().to_lowercase() == target_city
            {
                return Ok(region.id);
            }
        }
        Err(anyhow!(
            "region {} / {} not found in directory-api response; create it first via admin API",
            code,
            city
        ))
    }

    pub async fn register_node(&self, payload: &NodeRegisterRequest) -> Result<()> {
        let url = self.url_for("/nodes/register")?;
        let resp = self.http.post(url).json(payload).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("node register failed: {status} {body}"));
        }
        Ok(())
    }

    pub async fn send_heartbeat(&self, payload: &NodeHeartbeatRequest) -> Result<()> {
        let url = self.url_for("/nodes/heartbeat")?;
        let resp = self.http.post(url).json(payload).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("heartbeat failed: {status} {body}"));
        }
        Ok(())
    }
}
