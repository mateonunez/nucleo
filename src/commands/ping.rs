use clap::Args;
use serde_json::json;

use crate::client;
use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

#[derive(Args, Debug)]
pub struct PingArgs {
    /// Service name from configured URLs (e.g. "api", "auth")
    #[arg(long)]
    pub service: Option<String>,
    /// Direct URL to ping (overrides --service)
    #[arg(long)]
    pub url: Option<String>,
    /// Output format
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn handle(args: &PingArgs) -> Result<(), CliError> {
    let url = resolve_url(args)?;
    let http = client::build_client()?;
    let fmt = OutputFormat::from_str(&args.format);

    let start = std::time::Instant::now();
    let resp = client::send_with_retry(|| http.get(&url))
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("Ping failed: {e}")))?;
    let latency = start.elapsed();

    let status = resp.status().as_u16();
    let content_length = resp.content_length().unwrap_or(0);

    // Consume body to complete the request
    let _ = resp.bytes().await;

    let body = json!({
        "url": url,
        "status": status,
        "latency_ms": latency.as_millis(),
        "content_length": content_length,
    });

    println!("{}", formatter::format_value(&body, &fmt));
    Ok(())
}

fn resolve_url(args: &PingArgs) -> Result<String, CliError> {
    if let Some(ref url) = args.url {
        return Ok(url.clone());
    }

    if let Some(ref service) = args.service {
        let urls = config::load_service_urls()?;
        return config::require_url(&urls, service);
    }

    // Default: ping the first configured URL, or error
    let urls = config::load_service_urls()?;
    if let Some((name, url)) = urls.iter().next() {
        eprintln!("Pinging {name}: {url}");
        return Ok(url.clone());
    }

    Err(CliError::Validation(
        "No URL specified. Use --url <url> or --service <name>.".to_string(),
    ))
}
