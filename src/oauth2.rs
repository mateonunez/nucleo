use base64::Engine;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::error::CliError;
use crate::types::oauth2::OAuth2Config;

// ---------------------------------------------------------------------------
// PKCE
// ---------------------------------------------------------------------------

pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
}

/// Generate a PKCE code_verifier (43-128 chars) and code_challenge (SHA-256).
pub fn generate_pkce() -> PkceChallenge {
    use rand::Rng;

    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);

    let code_verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);

    let digest = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);

    PkceChallenge {
        code_verifier,
        code_challenge,
    }
}

// ---------------------------------------------------------------------------
// Authorization URL
// ---------------------------------------------------------------------------

/// Build the full authorization URL with all OAuth2 params.
pub fn build_authorize_url(
    config: &OAuth2Config,
    pkce: &PkceChallenge,
    state: &str,
    redirect_uri: &str,
) -> String {
    let scopes = config.scopes.join(" ");
    let mut params = vec![
        ("client_id", config.client_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", redirect_uri),
        ("state", state),
        ("code_challenge", &pkce.code_challenge),
        ("code_challenge_method", "S256"),
    ];
    if !scopes.is_empty() {
        params.push(("scope", &scopes));
    }

    let query = params
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencoded(v)))
        .collect::<Vec<_>>()
        .join("&");

    format!("{}?{}", config.authorize_url, query)
}

/// Minimal percent-encoding for URL query values.
fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Local callback server
// ---------------------------------------------------------------------------

/// Parsed callback result: (authorization_code, state).
pub type CallbackResult = (String, String);

/// Fixed port for the OAuth2 callback server.
///
/// Register `http://127.0.0.1:8888/callback` as the redirect URI in your provider dashboard.
/// If port 8888 is busy, falls back to an OS-assigned port — the redirect URI will then differ
/// from what is registered and the OAuth2 flow will fail.
const DEFAULT_CALLBACK_PORT: u16 = 8888;

/// Start a local HTTP server for the OAuth2 callback.
///
/// Tries port 8888 first; falls back to an OS-assigned port if 8888 is busy.
/// Returns the port and a receiver that yields `(code, state)` when the callback arrives.
pub async fn start_callback_server(
    redirect_path: &str,
) -> Result<(u16, oneshot::Receiver<CallbackResult>), CliError> {
    let listener = match TcpListener::bind(format!("127.0.0.1:{DEFAULT_CALLBACK_PORT}")).await {
        Ok(l) => l,
        Err(_) => TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to bind callback server: {e}")))?,
    };

    let port = listener
        .local_addr()
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to get local address: {e}")))?
        .port();

    let (tx, rx) = oneshot::channel();
    let path = redirect_path.to_string();

    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);

            // Parse "GET /callback?code=xxx&state=yyy HTTP/1.1"
            let result = parse_callback_request(&request, &path);

            let (status, body) = match &result {
                Ok(_) => (
                    "200 OK",
                    "<html><body><h1>Authentication successful!</h1><p>You can close this window.</p></body></html>",
                ),
                Err(msg) => (
                    "400 Bad Request",
                    // Leak is fine — this runs once
                    Box::leak(
                        format!("<html><body><h1>Error</h1><p>{msg}</p></body></html>")
                            .into_boxed_str(),
                    ) as &str,
                ),
            };

            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes()).await;
            let _ = stream.shutdown().await;

            if let Ok(cb) = result {
                let _ = tx.send(cb);
            }
        }
    });

    Ok((port, rx))
}

/// Parse the authorization code and state from the HTTP request line.
fn parse_callback_request(request: &str, expected_path: &str) -> Result<CallbackResult, String> {
    let first_line = request.lines().next().unwrap_or("");
    // "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid HTTP request".to_string());
    }

    let uri = parts[1];
    let (path, query) = uri.split_once('?').unwrap_or((uri, ""));

    if path != expected_path {
        return Err(format!("Unexpected path: {path}"));
    }

    let mut code = None;
    let mut state = None;

    for param in query.split('&') {
        if let Some((k, v)) = param.split_once('=') {
            match k {
                "code" => code = Some(v.to_string()),
                "state" => state = Some(v.to_string()),
                _ => {}
            }
        }
    }

    match (code, state) {
        (Some(c), Some(s)) => Ok((c, s)),
        (None, _) => Err("Missing 'code' parameter".to_string()),
        (_, None) => Err("Missing 'state' parameter".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Token exchange
// ---------------------------------------------------------------------------

/// Standard OAuth2 token response.
#[derive(Debug, serde::Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Seconds until token expires.
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub token_type: Option<String>,
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(
    http: &reqwest::Client,
    config: &OAuth2Config,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, CliError> {
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", &config.client_id),
        ("code_verifier", code_verifier),
    ];

    let secret_ref;
    if let Some(secret) = &config.client_secret {
        secret_ref = secret.as_str();
        form.push(("client_secret", secret_ref));
    }

    let resp = http
        .post(&config.token_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("Token exchange request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(CliError::Auth(format!(
            "Token exchange failed ({status}): {body}"
        )));
    }

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| CliError::Auth(format!("Failed to parse token response: {e}")))
}

/// Refresh an OAuth2 access token.
pub async fn refresh_oauth2(
    http: &reqwest::Client,
    config: &OAuth2Config,
    refresh_token: &str,
) -> Result<TokenResponse, CliError> {
    let mut form = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", &config.client_id),
    ];

    let secret_ref;
    if let Some(secret) = &config.client_secret {
        secret_ref = secret.as_str();
        form.push(("client_secret", secret_ref));
    }

    let resp = http
        .post(&config.token_url)
        .form(&form)
        .send()
        .await
        .map_err(|e| CliError::Auth(format!("OAuth2 refresh request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(CliError::Auth(format!(
            "OAuth2 refresh failed ({status}): {body}"
        )));
    }

    resp.json::<TokenResponse>()
        .await
        .map_err(|e| CliError::Auth(format!("Failed to parse refresh response: {e}")))
}

// ---------------------------------------------------------------------------
// Browser
// ---------------------------------------------------------------------------

/// Open a URL in the user's default browser.
pub fn open_browser(url: &str) -> bool {
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .status()
    } else {
        std::process::Command::new("xdg-open").arg(url).status()
    };

    result.is_ok_and(|s| s.success())
}

// ---------------------------------------------------------------------------
// State generation
// ---------------------------------------------------------------------------

/// Generate a random string for the OAuth2 `state` parameter (CSRF protection).
pub fn generate_state() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_verifier_length() {
        let pkce = generate_pkce();
        // 32 bytes → 43 base64url chars (no padding)
        assert_eq!(pkce.code_verifier.len(), 43);
        assert_eq!(pkce.code_challenge.len(), 43);
    }

    #[test]
    fn pkce_challenge_is_sha256_of_verifier() {
        let pkce = generate_pkce();
        let digest = Sha256::digest(pkce.code_verifier.as_bytes());
        let expected = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        assert_eq!(pkce.code_challenge, expected);
    }

    #[test]
    fn authorize_url_construction() {
        let config = OAuth2Config {
            client_id: "my-client".to_string(),
            authorize_url: "https://example.com/authorize".to_string(),
            token_url: "https://example.com/token".to_string(),
            scopes: vec!["read".to_string(), "write".to_string()],
            client_secret: None,
            redirect_path: "/callback".to_string(),
        };
        let pkce = PkceChallenge {
            code_verifier: "verifier".to_string(),
            code_challenge: "challenge".to_string(),
        };

        let url = build_authorize_url(&config, &pkce, "my-state", "http://localhost:8080/callback");
        assert!(url.starts_with("https://example.com/authorize?"));
        assert!(url.contains("client_id=my-client"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=my-state"));
        assert!(url.contains("code_challenge=challenge"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("scope=read%20write"));
    }

    #[test]
    fn parse_callback_valid() {
        let request = "GET /callback?code=abc123&state=xyz HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let (code, state) = parse_callback_request(request, "/callback").unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz");
    }

    #[test]
    fn parse_callback_missing_code() {
        let request = "GET /callback?state=xyz HTTP/1.1\r\n\r\n";
        let err = parse_callback_request(request, "/callback").unwrap_err();
        assert!(err.contains("code"));
    }

    #[test]
    fn parse_callback_wrong_path() {
        let request = "GET /wrong?code=abc&state=xyz HTTP/1.1\r\n\r\n";
        let err = parse_callback_request(request, "/callback").unwrap_err();
        assert!(err.contains("Unexpected path"));
    }

    #[test]
    fn state_is_random() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2);
        assert_eq!(s1.len(), 43); // 32 bytes → 43 base64url
    }

    #[tokio::test]
    async fn callback_server_round_trip() {
        let (port, rx) = start_callback_server("/callback").await.unwrap();

        // Simulate browser redirect
        let client = reqwest::Client::new();
        let _ = client
            .get(format!(
                "http://127.0.0.1:{port}/callback?code=test-code&state=test-state"
            ))
            .send()
            .await;

        let (code, state) = rx.await.unwrap();
        assert_eq!(code, "test-code");
        assert_eq!(state, "test-state");
    }
}
