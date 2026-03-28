use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("{message}")]
    Api {
        code: u16,
        message: String,
        reason: String,
    },

    #[error("{0}")]
    Validation(String),

    #[error("{0}")]
    Auth(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[allow(dead_code)]
pub const EXIT_CODE_DOCUMENTATION: &[(i32, &str)] = &[
    (0, "Success"),
    (
        CliError::EXIT_CODE_API,
        "API error  — server returned an error response",
    ),
    (
        CliError::EXIT_CODE_AUTH,
        "Auth error — credentials missing or invalid",
    ),
    (
        CliError::EXIT_CODE_VALIDATION,
        "Validation — bad arguments or input",
    ),
    (CliError::EXIT_CODE_OTHER, "Internal   — unexpected failure"),
];

impl CliError {
    pub const EXIT_CODE_API: i32 = 1;
    pub const EXIT_CODE_AUTH: i32 = 2;
    pub const EXIT_CODE_VALIDATION: i32 = 3;
    pub const EXIT_CODE_OTHER: i32 = 5;

    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Api { .. } => Self::EXIT_CODE_API,
            CliError::Auth(_) => Self::EXIT_CODE_AUTH,
            CliError::Validation(_) => Self::EXIT_CODE_VALIDATION,
            CliError::Other(_) => Self::EXIT_CODE_OTHER,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            CliError::Api {
                code,
                message,
                reason,
            } => json!({
                "error": {
                    "code": code,
                    "message": message,
                    "reason": reason,
                }
            }),
            CliError::Validation(msg) => json!({
                "error": {
                    "code": 400,
                    "message": msg,
                    "reason": "validationError",
                }
            }),
            CliError::Auth(msg) => json!({
                "error": {
                    "code": 401,
                    "message": msg,
                    "reason": "authError",
                }
            }),
            CliError::Other(e) => json!({
                "error": {
                    "code": 500,
                    "message": format!("{e:#}"),
                    "reason": "internalError",
                }
            }),
        }
    }
}

pub fn print_error_json(err: &CliError) {
    let json = err.to_json();
    println!(
        "{}",
        serde_json::to_string_pretty(&json).unwrap_or_default()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_are_distinct() {
        let codes = [
            CliError::EXIT_CODE_API,
            CliError::EXIT_CODE_AUTH,
            CliError::EXIT_CODE_VALIDATION,
            CliError::EXIT_CODE_OTHER,
        ];
        let unique: std::collections::HashSet<i32> = codes.iter().copied().collect();
        assert_eq!(unique.len(), codes.len());
    }

    #[test]
    fn error_to_json_api() {
        let err = CliError::Api {
            code: 404,
            message: "Not Found".to_string(),
            reason: "notFound".to_string(),
        };
        let json = err.to_json();
        assert_eq!(json["error"]["code"], 404);
        assert_eq!(json["error"]["message"], "Not Found");
    }

    #[test]
    fn error_to_json_auth() {
        let err = CliError::Auth("Token expired".to_string());
        let json = err.to_json();
        assert_eq!(json["error"]["code"], 401);
    }

    #[test]
    fn error_to_json_validation() {
        let err = CliError::Validation("missing arg".to_string());
        assert_eq!(err.exit_code(), CliError::EXIT_CODE_VALIDATION);
    }
}
