use serde_json::json;

use crate::config;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

pub async fn handle(format: &str) -> Result<(), CliError> {
    let fmt = OutputFormat::from_str(format);

    // CLI version
    let version = env!("CARGO_PKG_VERSION");
    let config_dir = config::config_dir()?.display().to_string();

    // Auth status
    let auth_status = match config::load_credentials() {
        Ok(creds) => {
            if creds.is_expired() {
                "expired".to_string()
            } else if let Ok(payload) = creds.decode_payload() {
                let user = payload
                    .name
                    .as_deref()
                    .or(payload.username.as_deref())
                    .or(payload.email.as_deref())
                    .unwrap_or(&payload.sub);
                format!("authenticated as {user}")
            } else {
                "authenticated".to_string()
            }
        }
        Err(_) => "not authenticated".to_string(),
    };

    // Project context
    let ctx = config::load_context().unwrap_or_default();

    // Config
    let cfg = config::load_config().unwrap_or_default();
    let urls = config::load_service_urls().unwrap_or_default();

    // Plugins
    let plugin_dir = config::plugins_dir().unwrap_or_default();
    let plugin_count = std::fs::read_dir(&plugin_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir() && e.path().join("plugin.json").exists())
                .count()
        })
        .unwrap_or(0);

    let body = json!({
        "version": version,
        "config_dir": config_dir,
        "auth": auth_status,
        "project": {
            "project_id": ctx.project_id,
            "env_id": ctx.env_id,
            "api_key": ctx.api_key,
            "stage": ctx.stage,
        },
        "active_env": cfg.active_env,
        "urls_count": urls.len(),
        "urls": urls,
        "plugins_installed": plugin_count,
    });

    if format == "text" {
        // Human-friendly text output
        println!("nucleo v{version}");
        println!("  Config: {config_dir}");
        println!("  Auth:   {auth_status}");
        if let Some(ref pid) = ctx.project_id {
            println!("  Project: {pid}");
        }
        if let Some(ref eid) = ctx.env_id {
            println!("  Env:     {eid}");
        }
        if !cfg.active_env.is_empty() {
            println!("  Preset:  {}", cfg.active_env);
        }
        println!("  URLs:    {} configured", urls.len());
        for (name, url) in &urls {
            println!("    {name}: {url}");
        }
        println!("  Plugins: {plugin_count} installed");
    } else {
        println!("{}", formatter::format_value(&body, &fmt));
    }

    Ok(())
}
