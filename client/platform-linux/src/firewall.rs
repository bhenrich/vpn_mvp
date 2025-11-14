use std::sync::Arc;

use vpn_core::{CommandExecutor, CoreError};
use std::fs;

/// Manage nftables-based kill switch rules for a given interface.
/// Strategy:
/// - Create table `inet vpn_mvp` with chain `killswitch` hooking into output.
/// - Allow traffic via the VPN interface and to specified control-plane IPs.
/// - Drop all other egress traffic.
pub struct LinuxNftables {
	exec: Arc<dyn CommandExecutor>,
}

impl LinuxNftables {
	pub fn new(exec: Arc<dyn CommandExecutor>) -> Self {
		Self { exec }
	}

	pub async fn apply_killswitch(&self, iface: &str, allow_ips_v4: &[&str], allow_ips_v6: &[&str]) -> Result<(), CoreError> {
		let mut script = String::new();
		script.push_str("add table inet vpn_mvp\n");
		script.push_str("flush table inet vpn_mvp\n");
		script.push_str("add table inet vpn_mvp\n");
		script.push_str("add chain inet vpn_mvp killswitch { type filter hook output priority 0; policy accept; }\n");
		// Allow traffic going out via VPN iface
		script.push_str(&format!("add rule inet vpn_mvp killswitch oifname \"{}\" accept\n", iface));
		// Allow control-plane IPv4
		if !allow_ips_v4.is_empty() {
			let set_elems = allow_ips_v4.join(", ");
			script.push_str(&format!(
				"add rule inet vpn_mvp killswitch ip daddr {{ {} }} accept\n",
				set_elems
			));
		}
		// Allow control-plane IPv6
		if !allow_ips_v6.is_empty() {
			let set_elems = allow_ips_v6.join(", ");
			script.push_str(&format!(
				"add rule inet vpn_mvp killswitch ip6 daddr {{ {} }} accept\n",
				set_elems
			));
		}
		// Finally, drop everything else
		script.push_str("add rule inet vpn_mvp killswitch counter drop\n");

		// Write nftables script to a stable temp file path that remains valid
		// while `nft -f` is executing. Using a fixed filename avoids the issue
		// where a tempfile is deleted before nft can read it.
		let path = {
			let tmp_path = std::env::temp_dir().join("vpn_mvp_nft_killswitch.nft");
			fs::write(&tmp_path, script)
				.map_err(|e| CoreError::Other(format!("write nft script: {e}")))?;
			tmp_path.to_string_lossy().to_string()
		};

		let out = self.exec.run("nft", &["-f", &path]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	pub async fn remove_killswitch(&self) -> Result<(), CoreError> {
		// Removing the table removes all rules
		let out = self.exec.run("nft", &["delete", "table", "inet", "vpn_mvp"]).await?;
		if out.status != 0 {
			// Best-effort: if table didn't exist, ignore
		}
		Ok(())
	}

	pub async fn allow_uid(&self, uid: u32) -> Result<(), CoreError> {
		// Insert a rule to accept traffic owned by uid before the drop
		let out = self.exec.run("nft", &["add", "rule", "inet", "vpn_mvp", "killswitch", "meta", "skuid", &uid.to_string(), "accept"]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}

	pub async fn apply_killswitch_with_uids(
		&self,
		iface: &str,
		allow_ips_v4: &[&str],
		allow_ips_v6: &[&str],
		allow_uids: &[u32],
	) -> Result<(), CoreError> {
		let mut script = String::new();
		script.push_str("add table inet vpn_mvp\n");
		script.push_str("flush table inet vpn_mvp\n");
		script.push_str("add table inet vpn_mvp\n");
		script.push_str("add chain inet vpn_mvp killswitch { type filter hook output priority 0; policy accept; }\n");
		script.push_str(&format!("add rule inet vpn_mvp killswitch oifname \"{}\" accept\n", iface));
		if !allow_ips_v4.is_empty() {
			let set_elems = allow_ips_v4.join(", ");
			script.push_str(&format!("add rule inet vpn_mvp killswitch ip daddr {{ {} }} accept\n", set_elems));
		}
		if !allow_ips_v6.is_empty() {
			let set_elems = allow_ips_v6.join(", ");
			script.push_str(&format!("add rule inet vpn_mvp killswitch ip6 daddr {{ {} }} accept\n", set_elems));
		}
		for uid in allow_uids {
			script.push_str(&format!("add rule inet vpn_mvp killswitch meta skuid {} accept\n", uid));
		}
		script.push_str("add rule inet vpn_mvp killswitch counter drop\n");

		let path = {
			let tmp_path = std::env::temp_dir().join("vpn_mvp_nft_killswitch_uids.nft");
			fs::write(&tmp_path, script)
				.map_err(|e| CoreError::Other(format!("write nft script: {e}")))?;
			tmp_path.to_string_lossy().to_string()
		};

		let out = self.exec.run("nft", &["-f", &path]).await?;
		if out.status != 0 {
			return Err(CoreError::CommandFailed(out.stderr));
		}
		Ok(())
	}
}


