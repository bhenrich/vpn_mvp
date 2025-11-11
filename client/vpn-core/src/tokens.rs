use crate::CoreError;

const TOKEN_SERVICE: &str = "vpn_mvp_tokens";

pub struct TokenStore;

impl TokenStore {
	pub fn new() -> Self {
		Self
	}

	pub async fn save(&self, email: &str, access_token: &str, refresh_token: &str) -> Result<(), CoreError> {
		let email = email.to_owned();
		let access = access_token.to_owned();
		let refresh = refresh_token.to_owned();
		tokio::task::spawn_blocking(move || -> Result<(), CoreError> {
			let acc = keyring::Entry::new(TOKEN_SERVICE, &format!("access:{email}"))
				.map_err(|e| CoreError::Other(format!("keyring entry: {e}")))?;
			acc.set_password(&access).map_err(|e| CoreError::Other(format!("keyring set: {e}")))?;
			let refx = keyring::Entry::new(TOKEN_SERVICE, &format!("refresh:{email}"))
				.map_err(|e| CoreError::Other(format!("keyring entry: {e}")))?;
			refx.set_password(&refresh).map_err(|e| CoreError::Other(format!("keyring set: {e}")))?;
			Ok(())
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}

	pub async fn load_access(&self, email: &str) -> Result<Option<String>, CoreError> {
		let email = email.to_owned();
		tokio::task::spawn_blocking(move || -> Result<Option<String>, CoreError> {
			let acc = keyring::Entry::new(TOKEN_SERVICE, &format!("access:{email}"))
				.map_err(|e| CoreError::Other(format!("keyring entry: {e}")))?;
			match acc.get_password() {
				Ok(s) => Ok(Some(s)),
				Err(keyring::Error::NoEntry) => Ok(None),
				Err(e) => Err(CoreError::Other(format!("keyring get: {e}"))),
			}
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}
}


