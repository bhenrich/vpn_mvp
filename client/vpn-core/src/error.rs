use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
	#[error("unsupported platform")]
	UnsupportedPlatform,
	#[error("not found: {0}")]
	NotFound(String),
	#[error("already exists: {0}")]
	AlreadyExists(String),
	#[error("invalid input: {0}")]
	InvalidInput(String),
	#[error("permission denied")]
	PermissionDenied,
	#[error("command failed: {0}")]
	CommandFailed(String),
	#[error("io error: {0}")]
	Io(#[from] std::io::Error),
	#[error("{0}")]
	Other(String),
}


