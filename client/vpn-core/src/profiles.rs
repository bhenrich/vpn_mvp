use std::{fs, path::PathBuf};

use crate::{CoreError, ProfileStore, VpnProfile};

pub struct FileProfileStore {
	base_dir: PathBuf,
}

impl FileProfileStore {
	pub fn new(app_name: &str) -> Result<Self, CoreError> {
		let dir = dirs_next::config_dir()
			.ok_or_else(|| CoreError::Other("could not resolve config dir".into()))?
			.join(app_name)
			.join("profiles");
		Ok(Self { base_dir: dir })
	}

	fn file_for(&self, name: &str) -> PathBuf {
		self.base_dir.join(format!("{name}.json"))
	}

	/// Test-only constructor to specify a custom base directory.
	#[cfg(test)]
	pub fn with_base_dir_for_tests(dir: PathBuf) -> Self {
		Self { base_dir: dir }
	}
}

#[async_trait::async_trait]
impl ProfileStore for FileProfileStore {
	async fn save(&self, profile: &VpnProfile) -> Result<(), CoreError> {
		let dir = self.base_dir.clone();
		let file = self.file_for(&profile.name);
		let profile = profile.clone();
		tokio::task::spawn_blocking(move || -> Result<(), CoreError> {
			fs::create_dir_all(dir)?;
			let data = serde_json::to_vec_pretty(&profile).map_err(|e| CoreError::Other(e.to_string()))?;
			fs::write(file, data)?;
			Ok(())
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}

	async fn load(&self, name: &str) -> Result<VpnProfile, CoreError> {
		let path = self.file_for(name);
		tokio::task::spawn_blocking(move || -> Result<VpnProfile, CoreError> {
			let data = fs::read(path)?;
			let p = serde_json::from_slice(&data).map_err(|e| CoreError::Other(e.to_string()))?;
			Ok(p)
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}

	async fn delete(&self, name: &str) -> Result<(), CoreError> {
		let path = self.file_for(name);
		tokio::task::spawn_blocking(move || -> Result<(), CoreError> {
			let _ = fs::remove_file(path);
			Ok(())
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}

	async fn list(&self) -> Result<Vec<VpnProfile>, CoreError> {
		let dir = self.base_dir.clone();
		tokio::task::spawn_blocking(move || -> Result<Vec<VpnProfile>, CoreError> {
			let mut out = Vec::new();
			let entries = match fs::read_dir(&dir) {
				Ok(e) => e,
				Err(_) => return Ok(out),
			};
			for entry in entries {
				let entry = entry.map_err(CoreError::from)?;
				if entry.file_type().map_err(CoreError::from)?.is_file() {
					if let Some(ext) = entry.path().extension() {
						if ext == "json" {
							let data = fs::read(entry.path())?;
							if let Ok(p) = serde_json::from_slice::<VpnProfile>(&data) {
								out.push(p);
							}
						}
					}
				}
			}
			Ok(out)
		})
		.await
		.map_err(|e| CoreError::Other(format!("spawn blocking failed: {e}")))?
	}
}


