use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::consts::{APP_DIR, APP_PREFIX};
use crate::error::CliError;
use crate::types::auth::Credentials;
use crate::types::oauth2::OAuth2Config;
use crate::types::project::ProjectContext;

/// Service URLs are a dynamic map: any service name → URL.
pub type ServiceUrls = HashMap<String, String>;

// ---------------------------------------------------------------------------
// Directory helpers
// ---------------------------------------------------------------------------

pub fn config_dir() -> Result<PathBuf, CliError> {
    let dir = dirs::config_dir()
        .ok_or_else(|| CliError::Other(anyhow::anyhow!("Could not determine config directory")))?
        .join(APP_DIR);

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to create config dir: {e}")))?;
    }

    Ok(dir)
}

fn credentials_path() -> Result<PathBuf, CliError> {
    Ok(config_dir()?.join("credentials.json"))
}

fn context_path() -> Result<PathBuf, CliError> {
    Ok(config_dir()?.join("context.json"))
}

fn config_json_path() -> Result<PathBuf, CliError> {
    Ok(config_dir()?.join("config.json"))
}

/// Directory where plugins are installed.
///
/// Priority: `config.json plugins.directory` → `~/.config/<app>/plugins/`.
pub fn plugins_dir() -> Result<PathBuf, CliError> {
    let dir = match load_config() {
        Ok(cfg) if cfg.plugins.directory.is_some() => PathBuf::from(cfg.plugins.directory.unwrap()),
        _ => config_dir()?.join("plugins"),
    };
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to create plugins dir: {e}")))?;
    }
    Ok(dir)
}

// ---------------------------------------------------------------------------
// Credentials (JWT storage)
// ---------------------------------------------------------------------------

/// Load credentials. Priority: <PREFIX>_TOKEN env → credentials.json file.
pub fn load_credentials() -> Result<Credentials, CliError> {
    let token_var = format!("{APP_PREFIX}_TOKEN");

    // Priority 1: environment variable (access_token only – no refresh possible)
    if let Ok(token) = std::env::var(&token_var) {
        if !token.is_empty() {
            return Ok(Credentials {
                access_token: token,
                refresh_token: String::new(),
                expires: i64::MAX,
                permissions: vec![],
                auth_method: "basic".to_string(),
                scopes: vec![],
            });
        }
    }

    // Priority 2: credentials file
    let path = credentials_path()?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| CliError::Auth(format!("Failed to read credentials file: {e}")))?;
        let creds: Credentials = serde_json::from_str(&content)
            .map_err(|e| CliError::Auth(format!("Invalid credentials file: {e}")))?;
        return Ok(creds);
    }

    Err(CliError::Auth(format!(
        "Not authenticated. Run `{} auth login` or set {token_var}.",
        crate::consts::APP_BIN
    )))
}

/// Convenience: load just the access_token string.
#[allow(dead_code)]
pub fn load_token() -> Result<String, CliError> {
    Ok(load_credentials()?.access_token)
}

pub fn save_credentials(creds: &Credentials) -> Result<(), CliError> {
    let path = credentials_path()?;
    let content = serde_json::to_string_pretty(creds)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to serialize credentials: {e}")))?;
    std::fs::write(&path, content)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to save credentials: {e}")))?;
    Ok(())
}

pub fn remove_credentials() -> Result<(), CliError> {
    let path = credentials_path()?;
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to remove credentials: {e}")))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Project context
// ---------------------------------------------------------------------------

/// Load project context. Priority: env vars > context.json > default.
pub fn load_context() -> Result<ProjectContext, CliError> {
    let path = context_path()?;
    let mut ctx = if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read context: {e}")))?;
        serde_json::from_str(&content)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Invalid context file: {e}")))?
    } else {
        ProjectContext::default()
    };

    // Env var overrides using the configured prefix
    let overrides = [
        ("project_id", format!("{APP_PREFIX}_PROJECT_ID")),
        ("env_id", format!("{APP_PREFIX}_ENV_ID")),
        ("api_key", format!("{APP_PREFIX}_API_KEY")),
        ("stage", format!("{APP_PREFIX}_STAGE")),
    ];

    for (field, var_name) in &overrides {
        if let Ok(val) = std::env::var(var_name) {
            if !val.is_empty() {
                match *field {
                    "project_id" => ctx.project_id = Some(val),
                    "env_id" => ctx.env_id = Some(val),
                    "api_key" => ctx.api_key = Some(val),
                    "stage" => ctx.stage = Some(val),
                    _ => {}
                }
            }
        }
    }

    Ok(ctx)
}

pub fn save_context(ctx: &ProjectContext) -> Result<(), CliError> {
    let path = context_path()?;
    let content = serde_json::to_string_pretty(ctx)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to serialize context: {e}")))?;
    std::fs::write(&path, content)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to save context: {e}")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Service URLs (config.toml)
// ---------------------------------------------------------------------------

fn default_active_env() -> String {
    String::new()
}

/// Plugin system settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginsConfig {
    /// Custom plugin directory (overrides default `~/.config/<app>/plugins`).
    pub directory: Option<String>,
    /// Plugin registries (schema only — not yet used for remote installs).
    #[serde(default)]
    pub registries: Vec<PluginRegistry>,
}

/// A plugin registry entry (schema only — remote install not yet implemented).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    pub name: String,
    pub url: String,
    pub token: Option<String>,
}

/// An environment preset: either the legacy flat URL map or the new structured format.
///
/// The `#[serde(untagged)]` attribute allows both formats in config.json:
/// - Legacy: `{ "auth": "https://...", "api": "https://..." }`
/// - Full:   `{ "urls": { ... }, "auth_method": "oauth2", "oauth2": { ... } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PresetConfig {
    /// New structured format with auth_method and optional OAuth2 config.
    Full(EnvironmentPreset),
    /// Legacy: flat URL map (treated as urls-only with basic auth).
    Legacy(ServiceUrls),
}

impl PresetConfig {
    /// Normalize to `EnvironmentPreset` regardless of format.
    pub fn into_preset(self) -> EnvironmentPreset {
        match self {
            PresetConfig::Full(preset) => preset,
            PresetConfig::Legacy(urls) => EnvironmentPreset {
                urls,
                auth_method: "basic".to_string(),
                oauth2: None,
            },
        }
    }

    /// Get service URLs regardless of format.
    pub fn urls(&self) -> &ServiceUrls {
        match self {
            PresetConfig::Full(preset) => &preset.urls,
            PresetConfig::Legacy(urls) => urls,
        }
    }
}

/// Structured environment preset with auth configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentPreset {
    /// Service URLs (dynamic map: service name → URL).
    pub urls: ServiceUrls,
    /// Auth method: "basic" (username/password) or "oauth2".
    #[serde(default = "default_basic")]
    pub auth_method: String,
    /// OAuth2 configuration (required when auth_method is "oauth2").
    #[serde(default)]
    pub oauth2: Option<OAuth2Config>,
}

fn default_basic() -> String {
    "basic".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Service URLs (dynamic map: service name → URL).
    #[serde(default)]
    pub urls: ServiceUrls,
    /// Name of the active environment preset.
    #[serde(default = "default_active_env")]
    pub active_env: String,
    /// User-defined environment presets.
    #[serde(default)]
    pub presets: HashMap<String, PresetConfig>,
    /// Plugin system settings.
    #[serde(default)]
    pub plugins: PluginsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            urls: ServiceUrls::new(),
            active_env: default_active_env(),
            presets: HashMap::new(),
            plugins: PluginsConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Environment presets
// ---------------------------------------------------------------------------

/// Available preset names (read from config.json).
pub fn env_preset_names() -> Vec<String> {
    match load_config() {
        Ok(cfg) => {
            let mut names: Vec<String> = cfg.presets.keys().cloned().collect();
            names.sort();
            names
        }
        Err(_) => Vec::new(),
    }
}

/// Return service URLs for a named preset, or `None` if not found.
pub fn env_preset(name: &str) -> Option<ServiceUrls> {
    load_config()
        .ok()?
        .presets
        .get(name)
        .map(|p| p.urls().clone())
}

/// Load the active environment preset as a structured `EnvironmentPreset`.
pub fn load_active_preset() -> Result<EnvironmentPreset, CliError> {
    let config = load_config()?;
    if config.active_env.is_empty() {
        return Err(CliError::Validation(format!(
            "No active environment set. Run `{} config env use <preset>`.",
            crate::consts::APP_BIN
        )));
    }
    config
        .presets
        .get(&config.active_env)
        .map(|p| p.clone().into_preset())
        .ok_or_else(|| {
            CliError::Validation(format!(
                "Preset '{}' not found in config.",
                config.active_env
            ))
        })
}

/// Load OAuth2 configuration from the active preset.
pub fn load_oauth2_config() -> Result<OAuth2Config, CliError> {
    let preset = load_active_preset()?;
    preset.oauth2.ok_or_else(|| {
        CliError::Validation(
            "No OAuth2 configuration in active preset. Add an 'oauth2' block to your preset in config.json.".to_string()
        )
    })
}

/// Load service URLs with env var overrides.
pub fn load_service_urls() -> Result<ServiceUrls, CliError> {
    let mut urls = load_config()?.urls;

    // Env var overrides: <PREFIX>_<KEY>_URL
    let keys: Vec<String> = urls.keys().cloned().collect();
    for key in keys {
        let env_var = format!(
            "{}_{}_URL",
            APP_PREFIX,
            key.to_uppercase().replace('-', "_")
        );
        if let Ok(val) = std::env::var(&env_var) {
            if !val.is_empty() {
                urls.insert(key, val);
            }
        }
    }

    Ok(urls)
}

/// Get a URL by service name, or return a clear error.
pub fn require_url(urls: &ServiceUrls, key: &str) -> Result<String, CliError> {
    urls.get(key).cloned().ok_or_else(|| {
        CliError::Validation(format!(
            "No '{key}' URL configured. Run `{} config set urls.{key} <url>` or set {}_{}_URL.",
            crate::consts::APP_BIN,
            APP_PREFIX,
            key.to_uppercase().replace('-', "_")
        ))
    })
}

pub fn load_config() -> Result<AppConfig, CliError> {
    let path = config_json_path()?;
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read config.json: {e}")))?;
        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Invalid config.json: {e}")))?;
        return Ok(config);
    }
    Ok(AppConfig::default())
}

pub fn save_config(config: &AppConfig) -> Result<(), CliError> {
    let path = config_json_path()?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to serialize config: {e}")))?;
    std::fs::write(&path, content)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to save config.json: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_preset_deserializes() {
        let json = r#"{
            "urls": {},
            "active_env": "dev",
            "presets": {
                "dev": {
                    "auth": "https://auth.example.com",
                    "api": "https://api.example.com"
                }
            }
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        let preset = config.presets.get("dev").unwrap();
        let urls = preset.urls();
        assert_eq!(urls.get("auth").unwrap(), "https://auth.example.com");
        assert_eq!(urls.get("api").unwrap(), "https://api.example.com");
    }

    #[test]
    fn full_preset_deserializes() {
        let json = r#"{
            "urls": {},
            "active_env": "dev",
            "presets": {
                "dev": {
                    "urls": { "api": "https://api.spotify.com/v1" },
                    "auth_method": "oauth2",
                    "oauth2": {
                        "client_id": "abc",
                        "authorize_url": "https://accounts.spotify.com/authorize",
                        "token_url": "https://accounts.spotify.com/api/token",
                        "scopes": ["user-read-playback-state"]
                    }
                }
            }
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        let preset = config.presets.get("dev").unwrap().clone().into_preset();
        assert_eq!(preset.auth_method, "oauth2");
        assert!(preset.oauth2.is_some());
        let oauth2 = preset.oauth2.unwrap();
        assert_eq!(oauth2.client_id, "abc");
        assert_eq!(oauth2.scopes, vec!["user-read-playback-state"]);
    }

    #[test]
    fn mixed_presets_deserialize() {
        let json = r#"{
            "urls": {},
            "active_env": "dev",
            "presets": {
                "dev": {
                    "urls": { "api": "https://api.dev.example.com" },
                    "auth_method": "oauth2",
                    "oauth2": {
                        "client_id": "abc",
                        "authorize_url": "https://auth.example.com/authorize",
                        "token_url": "https://auth.example.com/token"
                    }
                },
                "legacy": {
                    "auth": "https://auth.old.example.com",
                    "api": "https://api.old.example.com"
                }
            }
        }"#;
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.presets.len(), 2);

        let dev = config.presets.get("dev").unwrap().clone().into_preset();
        assert_eq!(dev.auth_method, "oauth2");

        let legacy = config.presets.get("legacy").unwrap().clone().into_preset();
        assert_eq!(legacy.auth_method, "basic");
        assert_eq!(
            legacy.urls.get("auth").unwrap(),
            "https://auth.old.example.com"
        );
    }

    #[test]
    fn empty_config_deserializes() {
        let json = "{}";
        let config: AppConfig = serde_json::from_str(json).unwrap();
        assert!(config.presets.is_empty());
        assert!(config.active_env.is_empty());
    }
}

/// Set a dotted key (e.g. "urls.auth") in the config.
pub fn set_config_value(key: &str, value: &str) -> Result<(), CliError> {
    let path = config_json_path()?;
    let content = if path.exists() {
        std::fs::read_to_string(&path)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read config.json: {e}")))?
    } else {
        String::new()
    };

    let mut doc: serde_json::Value = if content.is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&content)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Invalid config.json: {e}")))?
    };

    // Navigate dotted path
    let parts: Vec<&str> = key.split('.').collect();
    if parts.is_empty() {
        return Err(CliError::Validation("Empty config key".to_string()));
    }

    let mut current = &mut doc;
    for part in &parts[..parts.len() - 1] {
        if !current.is_object() {
            return Err(CliError::Validation(format!(
                "Key '{part}' is not an object"
            )));
        }
        let obj = current.as_object_mut().unwrap();
        if !obj.contains_key(*part) {
            obj.insert(part.to_string(), serde_json::json!({}));
        }
        current = obj.get_mut(*part).unwrap();
    }

    let last_key = parts[parts.len() - 1];
    if let Some(obj) = current.as_object_mut() {
        obj.insert(
            last_key.to_string(),
            serde_json::Value::String(value.to_string()),
        );
    } else {
        return Err(CliError::Validation(format!("Cannot set key '{key}'")));
    }

    let output = serde_json::to_string_pretty(&doc)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to serialize config: {e}")))?;
    std::fs::write(&path, output)
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to save config.json: {e}")))?;

    Ok(())
}
