use crate::{CoreError, ConnectionState, VpnProfile};

#[async_trait::async_trait]
pub trait VpnAdapter: Send + Sync {
	async fn init(&self) -> Result<(), CoreError>;

	async fn add_profile(&self, profile: VpnProfile) -> Result<(), CoreError>;
	async fn remove_profile(&self, name: &str) -> Result<(), CoreError>;
	async fn list_profiles(&self) -> Result<Vec<VpnProfile>, CoreError>;

	async fn connect(&self, name: &str) -> Result<(), CoreError>;
	async fn disconnect(&self, name: &str) -> Result<(), CoreError>;
	async fn status(&self, name: &str) -> Result<ConnectionState, CoreError>;
}

#[async_trait::async_trait]
pub trait CredentialStore: Send + Sync {
	async fn set_password(&self, profile: &str, username: &str, password: &str) -> Result<(), CoreError>;
	async fn get_password(&self, profile: &str, username: &str) -> Result<Option<String>, CoreError>;
	async fn delete_password(&self, profile: &str, username: &str) -> Result<(), CoreError>;
}

#[async_trait::async_trait]
pub trait ProfileStore: Send + Sync {
	async fn save(&self, profile: &VpnProfile) -> Result<(), CoreError>;
	async fn load(&self, name: &str) -> Result<VpnProfile, CoreError>;
	async fn delete(&self, name: &str) -> Result<(), CoreError>;
	async fn list(&self) -> Result<Vec<VpnProfile>, CoreError>;
}

pub trait Logger: Send + Sync {
	fn debug(&self, message: &str);
	fn info(&self, message: &str);
	fn warn(&self, message: &str);
	fn error(&self, message: &str);
}

#[cfg(feature = "eap-tls")]
#[async_trait::async_trait]
pub trait CertificateStore: Send + Sync {
	async fn list_identities(&self) -> Result<Vec<String>, CoreError>;
	async fn find_by_ref(&self, reference: &str) -> Result<Option<String>, CoreError>;
}


