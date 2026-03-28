use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;

use crate::config;
use crate::consts;
use crate::error::CliError;
use crate::formatter::{self, OutputFormat};

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// Plugin manifest (`plugin.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub engine: PluginEngine,
    #[serde(default)]
    pub commands: HashMap<String, PluginCommandDef>,
    /// Minimum CLI version required (not enforced yet).
    pub cli_version: Option<String>,
    pub registry: Option<String>,
}

/// Engine block — tells the CLI how to invoke the plugin process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEngine {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}

/// Description of one plugin sub-command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommandDef {
    pub description: String,
}

// ---------------------------------------------------------------------------
// CLI enum
// ---------------------------------------------------------------------------

#[derive(Subcommand, Debug)]
pub enum PluginsCommand {
    /// List installed plugins
    List {
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Install a plugin from a local directory path
    Install {
        /// Path to the plugin directory (must contain plugin.json)
        source: String,
    },
    /// Remove an installed plugin
    Remove {
        /// Plugin name to remove
        name: String,
    },
    /// Show details of an installed plugin
    Info {
        /// Plugin name
        name: String,
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Reinstall all plugins from the repo's plugins/ directory (or a specific one)
    Upgrade {
        /// Specific plugin name to upgrade (default: all)
        name: Option<String>,
    },
    /// Run a plugin command (or use: nucleo plugins <name> <sub> [args...])
    #[command(external_subcommand)]
    External(Vec<String>),
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

pub async fn handle(cmd: &PluginsCommand) -> Result<(), CliError> {
    match cmd {
        PluginsCommand::List { format } => list(format).await,
        PluginsCommand::Install { source } => install(source).await,
        PluginsCommand::Remove { name } => remove(name).await,
        PluginsCommand::Info { name, format } => info(name, format).await,
        PluginsCommand::Upgrade { name } => upgrade(name.as_deref()).await,
        PluginsCommand::External(args) => run_external(args).await,
    }
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

fn discover_plugins() -> Result<Vec<(PluginManifest, PathBuf)>, CliError> {
    let dir = config::plugins_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read plugins dir: {e}")))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read plugin entry: {e}")))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("plugin.json");
        if !manifest_path.exists() {
            continue;
        }
        match load_manifest(&manifest_path) {
            Ok(manifest) => plugins.push((manifest, path)),
            Err(_) => {}
        }
    }

    plugins.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    Ok(plugins)
}

fn load_manifest(path: &Path) -> Result<PluginManifest, CliError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read {}: {e}", path.display())))?;
    let manifest: PluginManifest = serde_json::from_str(&content).map_err(|e| {
        CliError::Validation(format!("Invalid plugin.json at {}: {e}", path.display()))
    })?;
    Ok(manifest)
}

fn find_plugin(name: &str) -> Result<(PluginManifest, PathBuf), CliError> {
    let dir = config::plugins_dir()?.join(name);
    let manifest_path = dir.join("plugin.json");
    if !manifest_path.exists() {
        return Err(CliError::Validation(format!(
            "Plugin '{name}' is not installed. Run `{} plugins list` to see installed plugins.",
            consts::APP_BIN
        )));
    }
    let manifest = load_manifest(&manifest_path)?;
    Ok((manifest, dir))
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

fn build_plugin_env(manifest: &PluginManifest, plugin_dir: &Path) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = HashMap::new();
    let prefix = consts::APP_PREFIX;

    // Plugin-specific vars
    env.insert(
        format!("{prefix}_PLUGIN_DIR"),
        plugin_dir.to_string_lossy().to_string(),
    );
    env.insert(format!("{prefix}_PLUGIN_NAME"), manifest.name.clone());

    // Tell plugins what prefix to use for env vars
    env.insert("CLI_ENV_PREFIX".to_string(), prefix.to_string());

    // Auth token (best-effort)
    if let Ok(token) = config::load_token() {
        env.insert(format!("{prefix}_TOKEN"), token);
    }

    // Service URLs (dynamic)
    if let Ok(urls) = config::load_service_urls() {
        for (key, val) in &urls {
            env.insert(
                format!("{}_{}_URL", prefix, key.to_uppercase().replace('-', "_")),
                val.clone(),
            );
        }
    }

    // Project context (best-effort)
    if let Ok(ctx) = config::load_context() {
        if let Some(v) = ctx.project_id {
            env.insert(format!("{prefix}_PROJECT_ID"), v);
        }
        if let Some(v) = ctx.env_id {
            env.insert(format!("{prefix}_ENV_ID"), v);
        }
        if let Some(v) = ctx.api_key {
            env.insert(format!("{prefix}_API_KEY"), v);
        }
        if let Some(v) = ctx.stage {
            env.insert(format!("{prefix}_STAGE"), v);
        }
    }

    // Extra env from manifest
    if let Some(extra) = &manifest.engine.env {
        for (k, v) in extra {
            env.insert(k.clone(), v.clone());
        }
    }

    env
}

fn resolve_engine_command(engine: &PluginEngine, plugin_dir: &Path) -> String {
    if engine.command.starts_with("./") || engine.command.starts_with("../") {
        plugin_dir
            .join(&engine.command)
            .to_string_lossy()
            .to_string()
    } else {
        engine.command.clone()
    }
}

async fn execute_plugin(
    manifest: &PluginManifest,
    plugin_dir: &Path,
    args: &[String],
) -> Result<(), CliError> {
    let cmd_path = resolve_engine_command(&manifest.engine, plugin_dir);
    let env_vars = build_plugin_env(manifest, plugin_dir);

    let mut full_args: Vec<&str> = manifest.engine.args.iter().map(|s| s.as_str()).collect();
    for a in args {
        full_args.push(a.as_str());
    }

    let mut cmd = Command::new(&cmd_path);
    cmd.args(&full_args);
    cmd.current_dir(plugin_dir);
    cmd.envs(env_vars);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let output = tokio::time::timeout(Duration::from_secs(120), cmd.status())
        .await
        .map_err(|_| {
            CliError::Other(anyhow::anyhow!(
                "Plugin '{}' timed out after 120 seconds",
                manifest.name
            ))
        })?
        .map_err(|e| {
            CliError::Other(anyhow::anyhow!(
                "Failed to execute plugin '{}': {e}",
                manifest.name
            ))
        })?;

    if !output.success() {
        let code = output.code().unwrap_or(5);
        return match code {
            1 => Err(CliError::Api {
                code: 500,
                message: format!("Plugin '{}' returned API error", manifest.name),
                reason: "pluginError".to_string(),
            }),
            2 => Err(CliError::Auth(format!(
                "Plugin '{}' returned auth error",
                manifest.name
            ))),
            3 => Err(CliError::Validation(format!(
                "Plugin '{}' returned validation error",
                manifest.name
            ))),
            _ => Err(CliError::Other(anyhow::anyhow!(
                "Plugin '{}' exited with code {code}",
                manifest.name
            ))),
        };
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Sub-commands
// ---------------------------------------------------------------------------

async fn list(format: &str) -> Result<(), CliError> {
    let plugins = discover_plugins()?;

    let items: Vec<serde_json::Value> = plugins
        .iter()
        .map(|(m, dir)| {
            json!({
                "name": m.name,
                "version": m.version,
                "description": m.description,
                "author": m.author,
                "engine": m.engine.command,
                "commands": m.commands.keys().collect::<Vec<_>>(),
                "path": dir.to_string_lossy(),
            })
        })
        .collect();

    let body = json!({ "plugins": items });
    println!(
        "{}",
        formatter::format_value(&body, &OutputFormat::from_str(format))
    );
    Ok(())
}

async fn install(source: &str) -> Result<(), CliError> {
    let source_path = PathBuf::from(source);
    let manifest_path = source_path.join("plugin.json");

    if !manifest_path.exists() {
        return Err(CliError::Validation(format!(
            "No plugin.json found at {}. A valid plugin directory must contain a plugin.json manifest.",
            manifest_path.display()
        )));
    }

    let manifest = load_manifest(&manifest_path)?;
    let dest = config::plugins_dir()?.join(&manifest.name);

    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| {
            CliError::Other(anyhow::anyhow!(
                "Failed to remove existing plugin '{}': {e}",
                manifest.name
            ))
        })?;
    }

    copy_dir_recursive(&source_path, &dest)?;

    let body = json!({
        "installed": {
            "name": manifest.name,
            "version": manifest.version,
            "description": manifest.description,
            "path": dest.to_string_lossy(),
        }
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&body).unwrap_or_default()
    );
    Ok(())
}

async fn remove(name: &str) -> Result<(), CliError> {
    let dir = config::plugins_dir()?.join(name);
    if !dir.exists() {
        return Err(CliError::Validation(format!(
            "Plugin '{name}' is not installed."
        )));
    }

    std::fs::remove_dir_all(&dir)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to remove plugin '{name}': {e}")))?;

    let body = json!({ "removed": { "name": name } });
    println!(
        "{}",
        serde_json::to_string_pretty(&body).unwrap_or_default()
    );
    Ok(())
}

async fn info(name: &str, format: &str) -> Result<(), CliError> {
    let (manifest, dir) = find_plugin(name)?;

    let commands_detail: serde_json::Value = manifest
        .commands
        .iter()
        .map(|(k, v)| (k.clone(), json!({ "description": v.description })))
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    let body = json!({
        "name": manifest.name,
        "version": manifest.version,
        "description": manifest.description,
        "author": manifest.author,
        "license": manifest.license,
        "engine": {
            "command": manifest.engine.command,
            "args": manifest.engine.args,
        },
        "commands": commands_detail,
        "cli_version": manifest.cli_version,
        "registry": manifest.registry,
        "path": dir.to_string_lossy(),
    });
    println!(
        "{}",
        formatter::format_value(&body, &OutputFormat::from_str(format))
    );
    Ok(())
}

async fn upgrade(name: Option<&str>) -> Result<(), CliError> {
    let source_dir = find_repo_plugins_dir()?;
    let installed = discover_plugins()?;
    if installed.is_empty() {
        return Err(CliError::Validation(format!(
            "No plugins installed. Use `{} plugins install <path>` first.",
            consts::APP_BIN
        )));
    }

    let mut upgraded = Vec::new();
    let mut skipped = Vec::new();

    let targets: Vec<&str> = match name {
        Some(n) => {
            if !installed.iter().any(|(m, _)| m.name == n) {
                return Err(CliError::Validation(format!(
                    "Plugin '{n}' is not installed. Run `{} plugins list` to see installed plugins.",
                    consts::APP_BIN
                )));
            }
            vec![n]
        }
        None => installed.iter().map(|(m, _)| m.name.as_str()).collect(),
    };

    for plugin_name in targets {
        let candidate = source_dir.join(plugin_name);
        let manifest_path = candidate.join("plugin.json");
        if !manifest_path.exists() {
            skipped.push(json!({
                "name": plugin_name,
                "reason": format!("No source found at {}", candidate.display()),
            }));
            continue;
        }

        let manifest = load_manifest(&manifest_path)?;
        let dest = config::plugins_dir()?.join(&manifest.name);

        if dest.exists() {
            std::fs::remove_dir_all(&dest).map_err(|e| {
                CliError::Other(anyhow::anyhow!(
                    "Failed to remove existing plugin '{}': {e}",
                    manifest.name
                ))
            })?;
        }

        copy_dir_recursive(&candidate, &dest)?;
        upgraded.push(json!({
            "name": manifest.name,
            "version": manifest.version,
            "source": candidate.to_string_lossy(),
            "path": dest.to_string_lossy(),
        }));
    }

    let body = json!({
        "upgraded": upgraded,
        "skipped": skipped,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&body).unwrap_or_default()
    );
    Ok(())
}

fn find_repo_plugins_dir() -> Result<PathBuf, CliError> {
    let cwd = std::env::current_dir().unwrap_or_default().join("plugins");
    if cwd.is_dir() {
        return Ok(cwd);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent().and_then(|p| p.parent()) {
            let bin_rel = parent.join("plugins");
            if bin_rel.is_dir() {
                return Ok(bin_rel);
            }
        }
    }

    Err(CliError::Validation(format!(
        "Cannot find repo plugins/ directory. Run from the repo root or pass a path to `{} plugins install`.",
        consts::APP_BIN
    )))
}

async fn run_external(args: &[String]) -> Result<(), CliError> {
    if args.is_empty() {
        return Err(CliError::Validation(format!(
            "Usage: {} plugins <plugin-name> [subcommand] [args...]",
            consts::APP_BIN
        )));
    }

    let plugin_name = &args[0];
    let plugin_args = &args[1..];

    let (manifest, dir) = find_plugin(plugin_name)?;
    execute_plugin(&manifest, &dir, plugin_args).await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), CliError> {
    std::fs::create_dir_all(dst).map_err(|e| {
        CliError::Other(anyhow::anyhow!(
            "Failed to create directory {}: {e}",
            dst.display()
        ))
    })?;

    for entry in std::fs::read_dir(src).map_err(|e| {
        CliError::Other(anyhow::anyhow!(
            "Failed to read directory {}: {e}",
            src.display()
        ))
    })? {
        let entry = entry.map_err(|e| CliError::Other(anyhow::anyhow!("Dir entry error: {e}")))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                CliError::Other(anyhow::anyhow!(
                    "Failed to copy {} -> {}: {e}",
                    src_path.display(),
                    dst_path.display()
                ))
            })?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_manifest() {
        let json = r#"{
            "name": "test-plugin",
            "version": "0.1.0",
            "description": "A test plugin",
            "engine": {
                "command": "python3",
                "args": ["main.py"]
            },
            "commands": {
                "hello": { "description": "Say hello" }
            }
        }"#;
        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.engine.command, "python3");
        assert_eq!(manifest.engine.args, vec!["main.py"]);
        assert!(manifest.commands.contains_key("hello"));
    }

    #[test]
    fn parse_minimal_manifest() {
        let json = r#"{
            "name": "bare",
            "version": "1.0.0",
            "description": "Bare bones",
            "engine": { "command": "./run" }
        }"#;
        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "bare");
        assert!(manifest.engine.args.is_empty());
        assert!(manifest.commands.is_empty());
        assert!(manifest.author.is_none());
    }

    #[test]
    fn resolve_relative_engine_command() {
        let engine = PluginEngine {
            command: "./bin/run".to_string(),
            args: vec![],
            env: None,
        };
        let dir = Path::new("/home/user/.config/nucleo/plugins/foo");
        let resolved = resolve_engine_command(&engine, dir);
        assert!(resolved.contains("plugins/foo"));
        assert!(resolved.ends_with("bin/run"));
    }

    #[test]
    fn resolve_absolute_engine_command() {
        let engine = PluginEngine {
            command: "python3".to_string(),
            args: vec!["main.py".to_string()],
            env: None,
        };
        let dir = Path::new("/some/path");
        let resolved = resolve_engine_command(&engine, dir);
        assert_eq!(resolved, "python3");
    }

    #[test]
    fn manifest_with_extra_env() {
        let json = r#"{
            "name": "envtest",
            "version": "0.1.0",
            "description": "Test extra env",
            "engine": {
                "command": "node",
                "args": ["index.js"],
                "env": { "MY_VAR": "hello" }
            }
        }"#;
        let manifest: PluginManifest = serde_json::from_str(json).unwrap();
        let extra = manifest.engine.env.unwrap();
        assert_eq!(extra.get("MY_VAR").unwrap(), "hello");
    }
}
