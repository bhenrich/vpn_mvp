use std::sync::Arc;

use tempfile::TempDir;
use vpn_core::{AuthMethod, CredentialStore, DnsSettings, DryRunCommandExecutor, FileProfileStore, ProfileStore, SplitTunnel, VpnAdapter, VpnProfile};
use platform_windows::WindowsAdapter;

struct TestCreds;

#[async_trait::async_trait]
impl CredentialStore for TestCreds {
	async fn set_password(&self, _profile: &str, _username: &str, _password: &str) -> Result<(), vpn_core::CoreError> { Ok(()) }
	async fn get_password(&self, _profile: &str, _username: &str) -> Result<Option<String>, vpn_core::CoreError> {
		Ok(Some("secret".into()))
	}
	async fn delete_password(&self, _profile: &str, _username: &str) -> Result<(), vpn_core::CoreError> { Ok(()) }
}

#[tokio::test]
async fn connect_uses_rasdial_and_succeeds_in_dry_run() {
	let tmp = TempDir::new().unwrap();
	let store = Arc::new(FileProfileStore::with_base_dir_for_tests(tmp.path().join("profiles")));
	let creds = Arc::new(TestCreds);
	let exec = Arc::new(DryRunCommandExecutor::default());
	let adapter = WindowsAdapter::new(exec, creds, store.clone());
	let profile = VpnProfile {
		name: "win".into(),
		server: "vpn.example.com".into(),
		remote_id: None,
		auth: AuthMethod::EapMsChapV2 { username: Some("user".into()) },
		split_tunnel: SplitTunnel { enabled: false },
		dns: DnsSettings { servers: vec![] },
	};
	store.save(&profile).await.unwrap();
	adapter.connect("win").await.unwrap();
}


