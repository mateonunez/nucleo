use serde::{Deserialize, Serialize};

/// Active project/environment context persisted to context.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
}
