use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct PeerDirective {
    pub public_key: String,
    pub allowed_ips: Vec<String>,
    pub persistent_keepalive: Option<u16>,
    pub remove: bool,
}

impl PeerDirective {
    fn new() -> Self {
        Self {
            public_key: String::new(),
            allowed_ips: Vec::new(),
            persistent_keepalive: None,
            remove: false,
        }
    }
}

pub fn parse_peer_config(raw: &[u8]) -> Result<PeerDirective> {
    let text = std::str::from_utf8(raw)
        .map_err(|_| anyhow!("peer config is not valid UTF-8"))?
        .replace("\r\n", "\n");
    let mut directive = PeerDirective::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.eq_ignore_ascii_case("[peer]")
        {
            continue;
        }
        let (key, value) = match trimmed.split_once('=') {
            Some(pair) => pair,
            None => continue,
        };
        let key = key.trim().to_lowercase();
        let value = value.trim();
        match key.as_str() {
            "publickey" => directive.public_key = value.to_string(),
            "allowedips" => {
                directive.allowed_ips = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "persistentkeepalive" => {
                if let Ok(v) = value.parse::<u16>() {
                    directive.persistent_keepalive = Some(v);
                }
            }
            "remove" => {
                directive.remove = matches!(value.to_lowercase().as_str(), "true" | "1" | "yes");
            }
            _ => {}
        }
    }
    if directive.public_key.is_empty() {
        return Err(anyhow!("peer config missing PublicKey"));
    }
    if directive.allowed_ips.is_empty() && !directive.remove {
        return Err(anyhow!("peer config missing AllowedIPs"));
    }
    Ok(directive)
}
