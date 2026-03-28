use serde::{Deserialize, Serialize};

/// OAuth2 configuration stored in config.json environment presets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    /// OAuth2 client ID (registered with the provider).
    pub client_id: String,
    /// Authorization endpoint (e.g. "https://accounts.spotify.com/authorize").
    pub authorize_url: String,
    /// Token endpoint (e.g. "https://accounts.spotify.com/api/token").
    pub token_url: String,
    /// Requested scopes (e.g. ["user-read-playback-state"]).
    #[serde(default)]
    pub scopes: Vec<String>,
    /// Client secret — only for confidential clients. None for public clients (PKCE).
    #[serde(default)]
    pub client_secret: Option<String>,
    /// Redirect path for the local callback server (default: "/callback").
    #[serde(default = "default_redirect_path")]
    pub redirect_path: String,
}

fn default_redirect_path() -> String {
    "/callback".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal() {
        let json = r#"{
            "client_id": "abc",
            "authorize_url": "https://example.com/authorize",
            "token_url": "https://example.com/token"
        }"#;
        let config: OAuth2Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.client_id, "abc");
        assert!(config.scopes.is_empty());
        assert!(config.client_secret.is_none());
        assert_eq!(config.redirect_path, "/callback");
    }

    #[test]
    fn deserialize_full() {
        let json = r#"{
            "client_id": "abc",
            "authorize_url": "https://example.com/authorize",
            "token_url": "https://example.com/token",
            "scopes": ["read", "write"],
            "client_secret": "secret",
            "redirect_path": "/auth/callback"
        }"#;
        let config: OAuth2Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.scopes, vec!["read", "write"]);
        assert_eq!(config.client_secret.as_deref(), Some("secret"));
        assert_eq!(config.redirect_path, "/auth/callback");
    }
}
