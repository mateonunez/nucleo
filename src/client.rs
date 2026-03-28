use reqwest::header::{HeaderMap, HeaderValue};

use crate::config;
use crate::consts;
use crate::error::CliError;
use crate::types::auth::Credentials;

pub fn build_client() -> Result<reqwest::Client, CliError> {
    let mut headers = HeaderMap::new();
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");

    let user_agent = format!("{name}/{version}");
    if let Ok(header_value) = HeaderValue::from_str(&user_agent) {
        headers.insert(reqwest::header::USER_AGENT, header_value);
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to build HTTP client: {e}")))
}

const MAX_RETRIES: u32 = 3;
const TOKEN_REFRESH_MARGIN_SECS: i64 = 120;

/// Send a request with 429 retry logic.
pub async fn send_with_retry(
    build_request: impl Fn() -> reqwest::RequestBuilder,
) -> Result<reqwest::Response, reqwest::Error> {
    for attempt in 0..MAX_RETRIES {
        let resp = build_request().send().await?;

        if resp.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Ok(resp);
        }

        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1 << attempt);

        tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
    }

    build_request().send().await
}

/// Send an authenticated request with auto-refresh and 401 retry.
///
/// `build_request` receives the current access_token and returns a RequestBuilder.
pub async fn send_authenticated(
    http: &reqwest::Client,
    build_request: impl Fn(&str) -> reqwest::RequestBuilder,
) -> Result<reqwest::Response, CliError> {
    // Ensure we have fresh credentials
    let mut creds = config::load_credentials()?;

    // Auto-refresh if token expires soon
    if creds.expires_soon(TOKEN_REFRESH_MARGIN_SECS) && !creds.refresh_token.is_empty() {
        match refresh_token(http, &creds).await {
            Ok(new_creds) => {
                config::save_credentials(&new_creds)?;
                creds = new_creds;
            }
            Err(_) => {
                // Continue with current token — may still work or will get 401 below
            }
        }
    }

    // First attempt
    let resp = send_with_retry(|| build_request(&creds.access_token))
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("Request failed: {e}")))?;

    // On 401, try to refresh once and retry
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED && !creds.refresh_token.is_empty() {
        match refresh_token(http, &creds).await {
            Ok(new_creds) => {
                config::save_credentials(&new_creds)?;
                let retry_resp = send_with_retry(|| build_request(&new_creds.access_token))
                    .await
                    .map_err(|e| {
                        CliError::Other(anyhow::anyhow!("Request failed after refresh: {e}"))
                    })?;
                return Ok(retry_resp);
            }
            Err(_) => {
                return Err(CliError::Auth(format!(
                    "Session expired. Please run `{} auth login`.",
                    consts::APP_BIN
                )));
            }
        }
    }

    Ok(resp)
}

/// Refresh the access token using the refresh token.
async fn refresh_token(
    http: &reqwest::Client,
    creds: &Credentials,
) -> Result<Credentials, CliError> {
    let urls = config::load_service_urls()?;
    let auth_url = config::require_url(&urls, "auth")?;
    let url = format!("{auth_url}/refresh");

    let resp = http
        .post(&url)
        .query(&[("refresh_token", &creds.refresh_token)])
        .bearer_auth(&creds.access_token)
        .send()
        .await
        .map_err(|e| CliError::Auth(format!("Token refresh request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(CliError::Auth(format!(
            "Token refresh failed with status {}",
            resp.status()
        )));
    }

    let new_creds: Credentials = resp
        .json()
        .await
        .map_err(|e| CliError::Auth(format!("Failed to parse refresh response: {e}")))?;

    Ok(new_creds)
}

/// Helper: parse API error response into CliError.
pub async fn handle_api_response(resp: reqwest::Response) -> Result<serde_json::Value, CliError> {
    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to read response: {e}")))?;

    let body: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::Object(Default::default())
    } else {
        serde_json::from_slice(&bytes)
            .map_err(|e| CliError::Other(anyhow::anyhow!("Failed to parse response: {e}")))?
    };

    if !status.is_success() {
        let message = body["error"]["message"]
            .as_str()
            .or_else(|| body["message"].as_str())
            .unwrap_or("Unknown error")
            .to_string();
        let reason = body["error"]["reason"]
            .as_str()
            .unwrap_or("apiError")
            .to_string();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(CliError::Auth(message));
        }

        return Err(CliError::Api {
            code: status.as_u16(),
            message,
            reason,
        });
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_client_succeeds() {
        assert!(build_client().is_ok());
    }
}
