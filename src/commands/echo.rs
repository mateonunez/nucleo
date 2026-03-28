use clap::Args;
use serde_json::json;

use crate::client;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

const DEFAULT_ECHO_URL: &str = "https://httpbin.org/post";

#[derive(Args, Debug)]
pub struct EchoArgs {
    /// JSON data to POST
    #[arg(long)]
    pub data: Option<String>,
    /// URL to POST to (default: httpbin.org/post)
    #[arg(long)]
    pub url: Option<String>,
    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &EchoArgs) -> Result<(), CliError> {
    let url = args.url.as_deref().unwrap_or(DEFAULT_ECHO_URL);
    let fmt = OutputFormat::from_str(&args.format);

    let data: serde_json::Value = match &args.data {
        Some(d) => serde_json::from_str(d)
            .map_err(|e| CliError::Validation(format!("Invalid JSON in --data: {e}")))?,
        None => json!({}),
    };

    let http = client::build_client()?;

    let start = std::time::Instant::now();
    let resp =
        client::send_authenticated(&http, |token| http.post(url).bearer_auth(token).json(&data))
            .await?;
    let latency = start.elapsed();

    let status = resp.status().as_u16();
    let body = client::handle_api_response(resp).await?;

    let result = json!({
        "url": url,
        "status": status,
        "latency_ms": latency.as_millis(),
        "response": body,
    });

    println!("{}", formatter::format_value(&result, &fmt));
    Ok(())
}
