use std::fs;
use std::sync::Arc;

use tempfile::NamedTempFile;
use vpn_core::{CommandExecutor, CoreError};

/// Manage a simple PF-based kill switch.
/// Strategy (MVP, replaces active ruleset while enabled):
/// - Enable PF if disabled.
/// - Load a restrictive ruleset:
///   - set block-policy drop
///   - skip on lo0
///   - block all
///   - pass out on utunX keep state
///   - optionally pass to control-plane IPs (v4/v6)
/// - Revert by reloading /etc/pf.conf (best-effort).
pub struct MacPfFirewall {
    exec: Arc<dyn CommandExecutor>,
}

impl MacPfFirewall {
    pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
        Self { exec }
    }

    pub async fn apply_killswitch(
        &self,
        iface: &str,
        allow_ips_v4: &[&str],
        allow_ips_v6: &[&str],
    ) -> Result<(), CoreError> {
        let mut rules = String::new();
        rules.push_str("set block-policy drop\n");
        rules.push_str("set skip on lo0\n");
        rules.push_str("block all\n");
        // Allow traffic via the utun interface
        rules.push_str(&format!("pass out on {} keep state\n", iface));
        // Allow explicit control-plane IPs if provided
        for ip in allow_ips_v4 {
            rules.push_str(&format!("pass out quick to {} keep state\n", ip));
        }
        for ip in allow_ips_v6 {
            rules.push_str(&format!("pass out quick to {} keep state\n", ip));
        }

        let path: String = tokio::task::spawn_blocking(move || -> Result<String, CoreError> {
            let f = NamedTempFile::new().map_err(|e| CoreError::Other(format!("tempfile: {e}")))?;
            fs::write(f.path(), rules).map_err(|e| CoreError::Other(format!("write pf rules: {e}")))?;
            Ok(f.into_temp_path().to_path_buf().to_string_lossy().to_string())
        })
        .await
        .map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))??;

        // Enable PF
        let _ = self.exec.run("pfctl", &["-e"]).await?;
        // Load rules
        let out = self.exec.run("pfctl", &["-f", &path]).await?;
        if out.status != 0 {
            return Err(CoreError::CommandFailed(out.stderr));
        }
        Ok(())
    }

    pub async fn revert(&self) -> Result<(), CoreError> {
        // Best-effort revert to system rules
        let out = self.exec.run("pfctl", &["-f", "/etc/pf.conf"]).await?;
        if out.status != 0 {
            // If fail, try disabling PF (last resort)
            let _ = self.exec.run("pfctl", &["-d"]).await?;
        }
        Ok(())
    }
}


