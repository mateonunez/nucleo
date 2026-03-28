use crate::error::CliError;

pub async fn handle() -> Result<(), CliError> {
    crate::mcp::start().await
}
