use std::io::{self, Write};

use clap::Args;

use crate::config;
use crate::consts;
use crate::error::CliError;

#[derive(Args, Debug)]
pub struct SetupArgs {
    /// Username for authentication (skips interactive prompt)
    #[arg(long)]
    pub username: Option<String>,
    /// Password for authentication (skips interactive prompt)
    #[arg(long)]
    pub password: Option<String>,
    /// Environment preset name (skips interactive prompt)
    #[arg(long)]
    pub env: Option<String>,
    /// Only write Claude Desktop MCP config (skip auth/env)
    #[arg(long)]
    pub claude_desktop_only: bool,
    /// Write Claude Desktop MCP config automatically
    #[arg(long)]
    pub claude_desktop: bool,
    /// Verify current setup without modifying anything
    #[arg(long)]
    pub check: bool,
}

pub async fn handle(args: &SetupArgs) -> Result<(), CliError> {
    let app = consts::APP_NAME;
    println!("Welcome to {app} setup!\n");

    // --claude-desktop-only
    if args.claude_desktop_only {
        write_claude_desktop_config()?;
        println!("\nSetup complete! Restart Claude Desktop to use {app}.");
        return Ok(());
    }

    // --check
    if args.check {
        return run_check();
    }

    // ── Step 1/5: Environment ────────────────────────────────────────────
    println!("Step 1/5: Environment");
    let presets = config::env_preset_names();
    if presets.is_empty() {
        println!("  No presets defined in config.json. Skipping.");
    } else {
        let env_name = select_environment(args, &presets)?;
        super::config_cmd::handle(&crate::commands::config_cmd::ConfigCommand::Env {
            command: crate::commands::config_cmd::EnvConfigCommand::Use { preset: env_name },
        })
        .await?;
    }
    println!();

    // ── Step 2/5: Authentication ─────────────────────────────────────────
    println!("Step 2/5: Authentication");
    let urls = config::load_service_urls().unwrap_or_default();
    if urls.contains_key("auth") {
        let (username, password) = resolve_credentials(args)?;
        super::auth::handle(&crate::commands::auth::AuthCommand::Login {
            username,
            password: Some(password),
        })
        .await?;
    } else {
        println!("  No 'auth' URL configured. Skipping authentication.");
        println!("  Set it with: {app} config set urls.auth <url>");
    }
    println!();

    // ── Step 3/5: Project Context ────────────────────────────────────────
    println!("Step 3/5: Project Context");
    print!("  Project ID (or press Enter to skip): ");
    io::stdout().flush().ok();
    let mut project_input = String::new();
    io::stdin()
        .read_line(&mut project_input)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read input: {e}")))?;
    let project_id = project_input.trim();
    if !project_id.is_empty() {
        let ctx = crate::types::project::ProjectContext {
            project_id: Some(project_id.to_string()),
            ..Default::default()
        };
        config::save_context(&ctx)?;
        println!("  Project set: {project_id}");
    } else {
        println!("  Skipped.");
    }
    println!();

    // ── Step 4/5: Claude Desktop ─────────────────────────────────────────
    println!("Step 4/5: Claude Desktop Integration");
    let write_config = if args.claude_desktop {
        true
    } else {
        print!("  Add {app} MCP server to Claude Desktop? [Y/n]: ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read input: {e}")))?;
        let trimmed = input.trim().to_lowercase();
        trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
    };

    if write_config {
        write_claude_desktop_config()?;
    } else {
        println!("  Skipped.");
    }
    println!();

    // ── Step 5/5: Verification ───────────────────────────────────────────
    println!("Step 5/5: Verification");
    print_verification(write_config);
    println!();

    println!("Setup complete!");
    println!();
    println!("  Try: {app} status");
    println!("  Or:  {app} ping --url https://httpbin.org/get");

    Ok(())
}

fn select_environment(args: &SetupArgs, presets: &[String]) -> Result<String, CliError> {
    match &args.env {
        Some(e) => {
            config::env_preset(e).ok_or_else(|| {
                CliError::Validation(format!(
                    "Unknown preset '{}'. Available: {}",
                    e,
                    presets.join(", ")
                ))
            })?;
            Ok(e.clone())
        }
        None => {
            for (i, name) in presets.iter().enumerate() {
                let marker = if i == 0 { " (default)" } else { "" };
                println!("  {}. {}{}", i + 1, name, marker);
            }
            print!(
                "  Select [1-{}] or press Enter for default: ",
                presets.len()
            );
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read input: {e}")))?;
            let trimmed = input.trim();
            if trimmed.is_empty() {
                Ok(presets[0].clone())
            } else if let Ok(idx) = trimmed.parse::<usize>() {
                if idx >= 1 && idx <= presets.len() {
                    Ok(presets[idx - 1].clone())
                } else {
                    Ok(presets[0].clone())
                }
            } else {
                Ok(presets[0].clone())
            }
        }
    }
}

fn resolve_credentials(args: &SetupArgs) -> Result<(String, String), CliError> {
    let username = match &args.username {
        Some(u) => u.clone(),
        None => {
            print!("  Username: ");
            io::stdout().flush().ok();
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read input: {e}")))?;
            input.trim().to_string()
        }
    };

    let password = match &args.password {
        Some(p) => p.clone(),
        None => rpassword::prompt_password("  Password: ")
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read password: {e}")))?,
    };

    Ok((username, password))
}

fn write_claude_desktop_config() -> Result<(), CliError> {
    let app_bin = consts::APP_BIN;

    // Find the binary path
    let bin_path = which_binary().unwrap_or_else(|| app_bin.to_string());

    let config_dir = dirs::config_dir()
        .ok_or_else(|| CliError::Other(anyhow::anyhow!("Cannot determine config directory")))?;

    let claude_config_path = config_dir.join("Claude").join("claude_desktop_config.json");

    // Read existing config or start fresh
    let mut config: serde_json::Value = if claude_config_path.exists() {
        let content = std::fs::read_to_string(&claude_config_path).map_err(|e| {
            CliError::Other(anyhow::anyhow!("Failed to read Claude Desktop config: {e}"))
        })?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Ensure mcpServers object exists
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = serde_json::json!({});
    }

    // Add our server entry
    config["mcpServers"][app_bin] = serde_json::json!({
        "command": bin_path,
        "args": ["mcp"]
    });

    // Write back
    if let Some(parent) = claude_config_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CliError::Other(anyhow::anyhow!("Failed to create Claude config dir: {e}"))
        })?;
    }

    let output = serde_json::to_string_pretty(&config)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to serialize config: {e}")))?;
    std::fs::write(&claude_config_path, output).map_err(|e| {
        CliError::Other(anyhow::anyhow!(
            "Failed to write Claude Desktop config: {e}"
        ))
    })?;

    println!("  Claude Desktop config: {}", claude_config_path.display());
    Ok(())
}

fn which_binary() -> Option<String> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(consts::APP_BIN);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

fn run_check() -> Result<(), CliError> {
    let app = consts::APP_BIN;

    // Auth
    match config::load_credentials() {
        Ok(creds) => {
            if creds.is_expired() {
                println!("  Auth:    EXPIRED — run `{app} auth login`");
            } else {
                println!("  Auth:    OK");
            }
        }
        Err(_) => println!("  Auth:    NOT SET — run `{app} auth login`"),
    }

    // Context
    let ctx = config::load_context().unwrap_or_default();
    if ctx.project_id.is_some() {
        println!("  Project: OK ({})", ctx.project_id.as_deref().unwrap());
    } else {
        println!("  Project: NOT SET");
    }

    // Config
    let cfg = config::load_config().unwrap_or_default();
    println!(
        "  Env:     {}",
        if cfg.active_env.is_empty() {
            "(none)"
        } else {
            &cfg.active_env
        }
    );
    println!("  URLs:    {} configured", cfg.urls.len());

    // Plugins
    let plugin_dir = config::plugins_dir().unwrap_or_default();
    let count = std::fs::read_dir(&plugin_dir)
        .map(|e| {
            e.filter_map(|e| e.ok())
                .filter(|e| e.path().join("plugin.json").exists())
                .count()
        })
        .unwrap_or(0);
    println!("  Plugins: {count} installed");

    Ok(())
}

fn print_verification(mcp_configured: bool) {
    let app = consts::APP_BIN;

    match config::load_credentials() {
        Ok(creds) if !creds.is_expired() => println!("  Auth:    OK"),
        _ => println!("  Auth:    ISSUE — run `{app} auth login`"),
    }

    let ctx = config::load_context().unwrap_or_default();
    if ctx.project_id.is_some() {
        println!("  Project: OK");
    } else {
        println!("  Project: not set (optional)");
    }

    let urls = config::load_service_urls().unwrap_or_default();
    println!("  URLs:    {} configured", urls.len());

    if mcp_configured {
        println!("  MCP:     configured (restart Claude Desktop to load)");
    }
}
