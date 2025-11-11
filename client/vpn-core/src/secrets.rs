use crate::{CoreError, CredentialStore};

const SERVICE_NAME: &str = "vpn_mvp";

fn entry_account(profile: &str, username: &str) -> String {
	format!("{profile}/{username}")
}

pub struct OsKeyringCredentialStore;

#[async_trait::async_trait]
impl CredentialStore for OsKeyringCredentialStore {
	async fn set_password(&self, profile: &str, username: &str, password: &str) -> Result<(), CoreError> {
		let profile = profile.to_owned();
		let username = username.to_owned();
		let password = password.to_owned();
		tokio::task::spawn_blocking(move || -> Result<(), CoreError> {
			let account = entry_account(&profile, &username);
			let entry = keyring::Entry::new(SERVICE_NAME, &account)
				.map_err(|e| CoreError::Other(format!("keyring entry error: {e}")))?;
			entry
				.set_password(&password)
				.map_err(|e| CoreError::Other(format!("keyring set error: {e}")))
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}

	async fn get_password(&self, profile: &str, username: &str) -> Result<Option<String>, CoreError> {
		let profile = profile.to_owned();
		let username = username.to_owned();
		tokio::task::spawn_blocking(move || -> Result<Option<String>, CoreError> {
			let account = entry_account(&profile, &username);
			let entry = keyring::Entry::new(SERVICE_NAME, &account)
				.map_err(|e| CoreError::Other(format!("keyring entry error: {e}")))?;
			match entry.get_password() {
				Ok(p) => Ok(Some(p)),
				Err(keyring::Error::NoEntry) => Ok(None),
				Err(e) => Err(CoreError::Other(format!("keyring get error: {e}"))),
			}
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}

	async fn delete_password(&self, profile: &str, username: &str) -> Result<(), CoreError> {
		let profile = profile.to_owned();
		let username = username.to_owned();
		tokio::task::spawn_blocking(move || -> Result<(), CoreError> {
			let account = entry_account(&profile, &username);
			let entry = keyring::Entry::new(SERVICE_NAME, &account)
				.map_err(|e| CoreError::Other(format!("keyring entry error: {e}")))?;
			match entry.delete_password() {
				Ok(_) => Ok(()),
				Err(keyring::Error::NoEntry) => Ok(()),
				Err(e) => Err(CoreError::Other(format!("keyring delete error: {e}"))),
			}
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}
}


