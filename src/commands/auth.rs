use clap::Subcommand;

use crate::client::build_client;
use crate::config;
use crate::error::CliError;
use crate::types::auth::Credentials;

#[derive(Subcommand, Debug)]
pub enum AuthCommand {
    /// Authenticate with username and password
    Login {
        /// Username
        #[arg(long, short)]
        username: String,
        /// Password (omit to be prompted)
        #[arg(long, short)]
        password: Option<String>,
    },
    /// Remove stored credentials
    Logout,
    /// Print the current access token (for piping)
    Token,
}

pub async fn handle(cmd: &AuthCommand) -> Result<(), CliError> {
    match cmd {
        AuthCommand::Login { username, password } => login(username, password.as_deref()).await,
        AuthCommand::Logout => logout(),
        AuthCommand::Token => token(),
    }
}

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

async fn login(username: &str, password: Option<&str>) -> Result<(), CliError> {
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
