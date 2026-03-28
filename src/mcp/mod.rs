pub mod executor;
pub mod tools;

use rmcp::ServiceExt;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{Implementation, ServerInfo};
use rmcp::tool_handler;
use tools::NucleoServer;

use crate::error::CliError;

/// MCP ServerHandler implementation for NucleoServer.
#[tool_handler]
impl ServerHandler for NucleoServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.server_info = Implementation::new(crate::consts::APP_NAME, env!("CARGO_PKG_VERSION"));
        info.instructions = Some(
            "nucleo is a reusable CLI framework. Use the available tools to check \
             status, ping services, and list installed plugins. Extend with custom \
             tools by adding entries to src/mcp/tools.rs."
                .into(),
        );
        info
    }
}

/// Start the MCP server on stdio transport.
pub async fn start() -> Result<(), CliError> {
    let server = NucleoServer::new();
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let service = server
        .serve(transport)
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("MCP server error: {e}")))?;
    service
        .waiting()
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("MCP server stopped: {e}")))?;
    Ok(())
}
