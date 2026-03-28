use serde::{Deserialize, Serialize};

/// JWT credentials returned by the auth API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix timestamp (seconds) when the access token expires.
    pub expires: i64,
    /// Permissions list from the JWT payload.
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Auth method used to obtain these credentials: "basic" or "oauth2".
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    /// OAuth2 scopes granted with this token.
    #[serde(default)]
    pub scopes: Vec<String>,
}

fn default_auth_method() -> String {
    "basic".to_string()
}

/// Decoded JWT payload (generic — no domain-specific claims).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct JwtPayload {
    #[serde(default)]
    pub sub: String,
    #[serde(default)]
    pub exp: i64,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub permissions: Option<Vec<String>>,
}

impl Credentials {
    /// Decode the JWT payload from the access token (no signature verification).
    pub fn decode_payload(&self) -> Result<JwtPayload, String> {
        decode_jwt_payload(&self.access_token)
    }

    /// Returns true if the token expires within `margin_secs` seconds.
    pub fn expires_soon(&self, margin_secs: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.expires - now < margin_secs
    }

    pub fn is_expired(&self) -> bool {
        self.expires_soon(0)
    }

    #[allow(dead_code)]
    pub fn is_admin(&self) -> bool {
        self.permissions.iter().any(|p| p == "*" || p == "admin")
    }
}

/// Decode a JWT payload without verifying the signature.
/// JWTs are `header.payload.signature` — we only need the middle part.
pub fn decode_jwt_payload(token: &str) -> Result<JwtPayload, String> {
    use base64::Engine;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT format".to_string());
    }

    let payload_b64 = parts[1];
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| format!("Failed to decode JWT payload: {e}"))?;

    serde_json::from_slice(&bytes).map_err(|e| format!("Failed to parse JWT payload: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_jwt(payload_json: &str) -> String {
        use base64::Engine;
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json);
        let sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("fakesig");
        format!("{header}.{payload}.{sig}")
    }

    #[test]
    fn decode_jwt_extracts_fields() {
        let token = make_test_jwt(
            r#"{"sub":"user1","exp":9999999999,"email":"a@b.com","name":"User One","username":"user1","permissions":["admin"]}"#,
        );
        let payload = decode_jwt_payload(&token).unwrap();
        assert_eq!(payload.sub, "user1");
        assert_eq!(payload.name.as_deref(), Some("User One"));
        assert_eq!(payload.username.as_deref(), Some("user1"));
        let perms = payload.permissions.expect("permissions should be present");
        assert_eq!(perms, vec!["admin"]);
    }

    #[test]
    fn credentials_is_admin() {
        let creds = Credentials {
            access_token: String::new(),
            refresh_token: String::new(),
            expires: 0,
            permissions: vec!["admin".to_string()],
            auth_method: "basic".to_string(),
            scopes: vec![],
        };
        assert!(creds.is_admin());
    }

    #[test]
    fn credentials_expires_soon() {
        let creds = Credentials {
            access_token: String::new(),
            refresh_token: String::new(),
            expires: chrono::Utc::now().timestamp() + 60,
            permissions: vec![],
            auth_method: "basic".to_string(),
            scopes: vec![],
        };
        assert!(creds.expires_soon(120));
        assert!(!creds.expires_soon(30));
    }

    #[test]
    fn credentials_backward_compat() {
        // Old credentials.json without auth_method/scopes should deserialize fine
        let json = r#"{
            "access_token": "tok",
            "refresh_token": "ref",
            "expires": 9999999999,
            "permissions": ["read"]
        }"#;
        let creds: Credentials = serde_json::from_str(json).unwrap();
        assert_eq!(creds.auth_method, "basic");
        assert!(creds.scopes.is_empty());
    }
}
