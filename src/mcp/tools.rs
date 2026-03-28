use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::schemars::JsonSchema;
use rmcp::{tool, tool_router};
use serde::{Deserialize, Serialize};

use super::executor;

// ---------------------------------------------------------------------------
// Server struct
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct NucleoServer {
    pub tool_router: ToolRouter<Self>,
}

impl NucleoServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Run a CLI subprocess and return stdout (including errors).
    async fn run(&self, args: &[&str]) -> String {
        match executor::execute(args).await {
            Ok(r) if r.exit_code == 0 => r.stdout,
            Ok(r) => {
                if r.stdout.is_empty() {
                    format!("{{\"error\": {{\"message\": \"{}\"}}}}", r.stderr.trim())
                } else {
                    r.stdout
                }
            }
            Err(e) => format!("{{\"error\": {{\"message\": \"{e}\"}}}}"),
        }
    }

    /// Run with owned args (for tools that build arg vectors dynamically).
    async fn run_owned(&self, args: &[String]) -> String {
        let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.run(&str_args).await
    }
}

// ---------------------------------------------------------------------------
// Parameter structs
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct StatusParams {}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PingParams {
    /// Service name from configured URLs, or omit to require --url
    pub service: Option<String>,
    /// Direct URL to ping
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PluginsListParams {}

// ---------------------------------------------------------------------------
// Tool router
// ---------------------------------------------------------------------------

#[tool_router]
impl NucleoServer {
    /// Check CLI status: version, auth, project context, configured URLs.
    #[tool(
        name = "nucleo_status",
        description = "Check CLI status: version, auth, project context, configured URLs"
    )]
    async fn tool_status(&self, Parameters(_params): Parameters<StatusParams>) -> String {
        self.run(&["status", "--format", "json"]).await
    }

    /// Ping a configured service URL to check connectivity and measure latency.
    #[tool(
        name = "nucleo_ping",
        description = "Ping a service URL to check connectivity and measure latency"
    )]
    async fn tool_ping(&self, Parameters(params): Parameters<PingParams>) -> String {
        let mut args = vec![
            "ping".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        if let Some(ref service) = params.service {
            args.push("--service".to_string());
            args.push(service.clone());
        }
        if let Some(ref url) = params.url {
            args.push("--url".to_string());
            args.push(url.clone());
        }
        self.run_owned(&args).await
    }

    /// List installed plugins with their versions and commands.
    #[tool(
        name = "nucleo_plugins_list",
        description = "List installed plugins with their versions and commands"
    )]
    async fn tool_plugins_list(
        &self,
        Parameters(_params): Parameters<PluginsListParams>,
    ) -> String {
        self.run(&["plugins", "list", "--format", "json"]).await
    }
}
