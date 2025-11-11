use std::sync::Arc;

use vpn_core::{DryRunCommandExecutor, FileProfileStore, ProfileStore, VpnAdapter, VpnProfile, AuthMethod, SplitTunnel, DnsSettings};
use platform_linux::LinuxNmAdapter;

#[tokio::test]
async fn linux_nm_connect_disconnect_status_dry_run() {
	let exec = Arc::new(DryRunCommandExecutor::default());
	let store = Arc::new(FileProfileStore::new("vpn_mvp_test").unwrap());
	let adapter = LinuxNmAdapter::new(exec, store.clone());

	let profile = VpnProfile {
		name: "nm".into(),
		server: "vpn.example.com".into(),
		remote_id: None,
		auth: AuthMethod::EapMsChapV2 { username: None },
		split_tunnel: SplitTunnel { enabled: false },
		dns: DnsSettings { servers: vec![] },
	};
	store.save(&profile).await.unwrap();

	adapter.connect("nm").await.unwrap();
	adapter.status("nm").await.unwrap();
	adapter.disconnect("nm").await.unwrap();
}


