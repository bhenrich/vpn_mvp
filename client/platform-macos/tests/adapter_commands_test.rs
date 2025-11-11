use std::sync::Arc;

use vpn_core::{DryRunCommandExecutor, FileProfileStore, ProfileStore, VpnAdapter, VpnProfile, AuthMethod, SplitTunnel, DnsSettings};
use platform_macos::MacosAdapter;

#[tokio::test]
async fn macos_connect_disconnect_status_dry_run() {
	let exec = Arc::new(DryRunCommandExecutor::default());
	let store = Arc::new(FileProfileStore::new("vpn_mvp_test").unwrap());
	let creds = Arc::new(vpn_core::OsKeyringCredentialStore);
	let adapter = MacosAdapter::new(exec, creds, store.clone());

	let profile = VpnProfile {
		name: "mac".into(),
		server: "vpn.example.com".into(),
		remote_id: None,
		auth: AuthMethod::EapMsChapV2 { username: None },
		split_tunnel: SplitTunnel { enabled: false },
		dns: DnsSettings { servers: vec![] },
	};
	store.save(&profile).await.unwrap();

	adapter.connect("mac").await.unwrap();
	adapter.status("mac").await.unwrap();
	adapter.disconnect("mac").await.unwrap();
}


