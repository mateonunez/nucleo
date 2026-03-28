use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use crate::consts;

/// Result of executing a CLI subprocess.
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Find the CLI binary: PATH first, then current_exe() fallback.
fn resolve_cli_binary() -> std::ffi::OsString {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(consts::APP_BIN);
            if candidate.is_file() {
                return candidate.into_os_string();
            }
        }
    }
    std::env::current_exe()
        .map(|p| p.into_os_string())
        .unwrap_or_else(|_| consts::APP_BIN.into())
}

/// Execute a CLI command as a subprocess and capture output.
///
/// stdin is set to Stdio::null() to prevent the child from consuming
/// the parent's stdin (which is the MCP JSON-RPC transport).
pub async fn execute(args: &[&str]) -> Result<CommandResult, String> {
    let exe = resolve_cli_binary();

    let mut cmd = Command::new(&exe);
    cmd.args(args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = tokio::time::timeout(Duration::from_secs(30), cmd.output())
        .await
        .map_err(|_| "Command timed out after 30 seconds".to_string())?
        .map_err(|e| format!("Failed to execute {}: {e}", consts::APP_BIN))?;

    Ok(CommandResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}
