use std::sync::Arc;

use tempfile::TempDir;
use vpn_core::{FileProfileStore, ProfileStore, VpnProfile, AuthMethod, SplitTunnel, DnsSettings};

#[tokio::test]
async fn file_profile_store_crud() {
	let tmp = TempDir::new().unwrap();
	let store = FileProfileStore::with_base_dir_for_tests(tmp.path().join("profiles"));

	let profile = VpnProfile {
		name: "test".into(),
		server: "vpn.example.com".into(),
		remote_id: Some("example.com".into()),
		auth: AuthMethod::EapMsChapV2 { username: Some("user".into()) },
		split_tunnel: SplitTunnel { enabled: true },
		dns: DnsSettings { servers: vec!["1.1.1.1".into()] },
	};

	store.save(&profile).await.unwrap();

	let loaded = store.load("test").await.unwrap();
	assert_eq!(loaded.name, "test");
	assert_eq!(loaded.server, "vpn.example.com");

	let list = store.list().await.unwrap();
	assert_eq!(list.len(), 1);
	assert_eq!(list[0].name, "test");

	store.delete("test").await.unwrap();
	let list = store.list().await.unwrap();
	assert!(list.is_empty());
}


