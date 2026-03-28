use clap::Subcommand;

use crate::client::build_client;
use crate::config;
use crate::error::CliError;
use crate::oauth2;
use crate::types::auth::Credentials;

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    /// Authenticate with username/password or OAuth2
    Login {
        /// Username (required for basic auth, ignored for OAuth2)
        #[arg(long, short)]
        username: Option<String>,
        /// Password (omit to be prompted; ignored for OAuth2)
        #[arg(long, short)]
        password: Option<String>,
        /// Force OAuth2 authorization code flow
        #[arg(long)]
        oauth2: bool,
        /// Print the authorization URL instead of opening the browser
        #[arg(long)]
        no_browser: bool,
    },
    /// Remove stored credentials
    Logout,
    /// Print the current access token (for piping)
    Token,
}

pub async fn handle(cmd: &AuthCommand) -> Result<(), CliError> {
    match cmd {
        AuthCommand::Login {
            username,
            password,
            oauth2,
            no_browser,
        } => {
            if should_use_oauth2(*oauth2) {
                login_oauth2(*no_browser).await
            } else {
                let username = username.as_deref().ok_or_else(|| {
                    CliError::Validation(
                        "Username is required for basic auth. Use --username or --oauth2 for OAuth2 flow.".to_string(),
                    )
                })?;
                login_basic(username, password.as_deref()).await
            }
        }
        AuthCommand::Logout => logout(),
        AuthCommand::Token => token(),
    }
}

/// Determine whether to use OAuth2: explicit flag or active preset's auth_method.
fn should_use_oauth2(explicit_flag: bool) -> bool {
    if explicit_flag {
        return true;
    }
    config::load_active_preset()
        .map(|p| p.auth_method == "oauth2")
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Basic auth login (existing flow)
// ---------------------------------------------------------------------------

/// API response shape from POST /token
#[derive(serde::Deserialize)]
struct LoginResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires: Option<i64>,
    permissions: Option<LoginPermissions>,
}

#[derive(serde::Deserialize)]
struct LoginPermissions {
    #[serde(default)]
    can: Vec<String>,
}

async fn login_basic(username: &str, password: Option<&str>) -> Result<(), CliError> {
    let password = match password {
        Some(p) => p.to_string(),
        None => rpassword::prompt_password("Password: ")
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read password: {e}")))?,
    };

    let urls = config::load_service_urls()?;
    let auth_url = config::require_url(&urls, "auth")?;
    let url = format!("{auth_url}/token");

    let http = build_client()?;
    let resp = http
        .post(&url)
        .basic_auth(username, Some(&password))
        .send()
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("Login request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        let message = body["error"]["message"]
            .as_str()
            .or_else(|| body["message"].as_str())
            .unwrap_or("Invalid credentials")
            .to_string();
        return Err(CliError::Api {
            code: status.as_u16(),
            message,
            reason: "loginFailed".to_string(),
        });
    }

    let raw: LoginResponse = resp
        .json()
        .await
        .map_err(|e| CliError::Auth(format!("Failed to parse login response: {e}")))?;

    let creds = Credentials {
        access_token: raw.access_token.unwrap_or_default(),
        refresh_token: raw.refresh_token.unwrap_or_default(),
        expires: raw.expires.unwrap_or(i64::MAX),
        permissions: raw.permissions.map(|p| p.can).unwrap_or_default(),
        auth_method: "basic".to_string(),
        scopes: vec![],
    };

    config::save_credentials(&creds)?;

    if let Ok(payload) = creds.decode_payload() {
        let user = payload
            .name
            .as_deref()
            .or(payload.username.as_deref())
            .or(payload.email.as_deref())
            .unwrap_or(&payload.sub);
        println!("Logged in as {user}");
    } else {
        println!("Logged in successfully.");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// OAuth2 login (Authorization Code + PKCE)
// ---------------------------------------------------------------------------

const OAUTH2_CALLBACK_TIMEOUT_SECS: u64 = 120;

async fn login_oauth2(no_browser: bool) -> Result<(), CliError> {
    let oauth2_config = config::load_oauth2_config()?;

    // 1. Generate PKCE challenge and state
    let pkce = oauth2::generate_pkce();
    let state = oauth2::generate_state();

    // 2. Start local callback server
    let (port, rx) = oauth2::start_callback_server(&oauth2_config.redirect_path).await?;
    let redirect_uri = format!("http://127.0.0.1:{port}{}", oauth2_config.redirect_path);

    // 3. Build authorization URL
    let authorize_url =
        oauth2::build_authorize_url(&oauth2_config, &pkce, &state, &redirect_uri);

    // 4. Open browser or print URL
    if no_browser || !oauth2::open_browser(&authorize_url) {
        println!("Open this URL in your browser to authorize:\n");
        println!("  {authorize_url}\n");
    } else {
        println!("Opening browser for authorization...");
    }

    println!("Waiting for callback on http://127.0.0.1:{port}{}...", oauth2_config.redirect_path);

    // 5. Await callback
    let (code, returned_state) = tokio::time::timeout(
        std::time::Duration::from_secs(OAUTH2_CALLBACK_TIMEOUT_SECS),
        rx,
    )
    .await
    .map_err(|_| {
        CliError::Auth(format!(
            "OAuth2 callback timed out after {OAUTH2_CALLBACK_TIMEOUT_SECS}s. Try again."
        ))
    })?
    .map_err(|_| CliError::Auth("OAuth2 callback channel closed unexpectedly.".to_string()))?;

    // 6. Validate state
    if returned_state != state {
        return Err(CliError::Auth(
            "OAuth2 state mismatch — possible CSRF attack. Try again.".to_string(),
        ));
    }

    // 7. Exchange code for tokens
    let http = build_client()?;
    let token_resp =
        oauth2::exchange_code(&http, &oauth2_config, &code, &pkce.code_verifier, &redirect_uri)
            .await?;

    // 8. Build and save credentials
    let expires = token_resp.expires_in.map_or(i64::MAX, |secs| {
        chrono::Utc::now().timestamp() + secs
    });
    let scopes = token_resp
        .scope
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default();

    let creds = Credentials {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token.unwrap_or_default(),
        expires,
        permissions: vec![],
        auth_method: "oauth2".to_string(),
        scopes,
    };

    config::save_credentials(&creds)?;

    // 9. Display result
    if let Ok(payload) = creds.decode_payload() {
        let user = payload
            .name
            .as_deref()
            .or(payload.username.as_deref())
            .or(payload.email.as_deref())
            .unwrap_or(&payload.sub);
        println!("Logged in as {user} (OAuth2)");
    } else {
        println!("Logged in successfully (OAuth2).");
    }

    Ok(())
}

fn logout() -> Result<(), CliError> {
    config::remove_credentials()?;
    println!("Logged out.");
    Ok(())
}

fn token() -> Result<(), CliError> {
    let creds = config::load_credentials()?;
    print!("{}", creds.access_token);
    Ok(())
}
