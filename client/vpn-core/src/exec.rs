use crate::CoreError;

#[derive(Debug, Clone)]
pub struct CommandOutput {
	pub status: i32,
	pub stdout: String,
	pub stderr: String,
}

#[async_trait::async_trait]
pub trait CommandExecutor: Send + Sync {
	async fn run(&self, program: &str, args: &[&str]) -> Result<CommandOutput, CoreError> {
		self.run_with_env(program, args, &[]).await
	}
	async fn run_with_env(&self, program: &str, args: &[&str], env: &[(&str, &str)]) -> Result<CommandOutput, CoreError>;
}

pub struct RealCommandExecutor;

#[async_trait::async_trait]
impl CommandExecutor for RealCommandExecutor {
	async fn run_with_env(&self, program: &str, args: &[&str], env: &[(&str, &str)]) -> Result<CommandOutput, CoreError> {
		let mut cmd = tokio::process::Command::new(program);
		cmd.args(args);
		for (k, v) in env {
			cmd.env(k, v);
		}
		let output = cmd.output().await.map_err(CoreError::from)?;
		let status = output.status.code().unwrap_or_default();
		Ok(CommandOutput {
			status,
			stdout: String::from_utf8_lossy(&output.stdout).to_string(),
			stderr: String::from_utf8_lossy(&output.stderr).to_string(),
		})
	}
}

#[derive(Default)]
pub struct DryRunCommandExecutor;

#[async_trait::async_trait]
impl CommandExecutor for DryRunCommandExecutor {
	async fn run_with_env(&self, _program: &str, _args: &[&str], _env: &[(&str, &str)]) -> Result<CommandOutput, CoreError> {
		Ok(CommandOutput {
			status: 0,
			stdout: String::new(),
			stderr: String::new(),
		})
	}
}


