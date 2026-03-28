use clap::Subcommand;

use crate::config;
use crate::error::CliError;

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Manage environment presets
    Env {
        #[command(subcommand)]
        command: EnvConfigCommand,
    },
    /// Set a single configuration value
    ///
    /// KEY is a dotted path (e.g. `urls.auth`) or a bare service name
    /// (e.g. `auth`) which is expanded to `urls.<name>` automatically.
    Set {
        /// Config key  (e.g. `urls.api` or shorthand `api`)
        key: String,
        /// New value
        value: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum EnvConfigCommand {
    /// List available environment presets
    List,
    /// Switch to a named environment preset and write its URLs to config.json
    Use {
        /// Preset name (defined in config.json presets.<name>)
        preset: String,
    },
}

pub async fn handle(cmd: &ConfigCommand) -> Result<(), CliError> {
    match cmd {
        ConfigCommand::Show => show(),
        ConfigCommand::Env { command } => handle_env(command),
        ConfigCommand::Set { key, value } => set_value(key, value),
    }
}

fn show() -> Result<(), CliError> {
    let cfg = config::load_config()?;
    let json_str = serde_json::to_string_pretty(&cfg)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to format config: {e}")))?;
    println!("{json_str}");
    Ok(())
}

fn handle_env(cmd: &EnvConfigCommand) -> Result<(), CliError> {
    match cmd {
        EnvConfigCommand::List => list_presets(),
        EnvConfigCommand::Use { preset } => use_preset(preset),
    }
}

fn list_presets() -> Result<(), CliError> {
    let cfg = config::load_config()?;
    let active = &cfg.active_env;
    let names = config::env_preset_names()?;

    if names.is_empty() {
        println!("No presets defined. Add them to config.json under presets.<name>.");
        return Ok(());
    }

    for name in &names {
        if name == active {
            println!("{name} (active)");
        } else {
            println!("{name}");
        }
    }
    Ok(())
}

fn use_preset(preset: &str) -> Result<(), CliError> {
    let available = config::env_preset_names()?.join(", ");
    let urls = config::env_preset(preset).ok_or_else(|| {
        let hint = if available.is_empty() {
            "No presets defined. Add them to config.json under presets.<name>.".to_string()
        } else {
            format!("Available: {available}")
        };
        CliError::Validation(format!("Unknown preset '{preset}'. {hint}"))
    })?;

    let mut cfg = config::load_config().unwrap_or_default();
    cfg.urls = urls;
    cfg.active_env = preset.to_string();
    config::save_config(&cfg)?;
    println!("Active environment: {preset}");
    Ok(())
}

fn set_value(key: &str, value: &str) -> Result<(), CliError> {
    // Bare service name (no dot) → expand to urls.<name>
    let resolved = if key.contains('.') {
        key.to_string()
    } else {
        format!("urls.{key}")
    };
    config::set_config_value(&resolved, value)?;
    println!("config.{resolved} = {value}");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn set_value_expands_bare_key() {
        let key = "api";
        let resolved = if key.contains('.') {
            key.to_string()
        } else {
            format!("urls.{key}")
        };
        assert_eq!(resolved, "urls.api");
    }
}
