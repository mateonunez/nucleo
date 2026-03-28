use serde::Deserialize;

/// Generic paginated response wrapper.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PaginatedResponse<T> {
    #[serde(flatten)]
    pub items: std::collections::HashMap<String, serde_json::Value>,
    #[serde(default, alias = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(skip)]
    pub _marker: std::marker::PhantomData<T>,
}

/// Common pagination query parameters.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PaginationParams {
    pub page_size: Option<u32>,
    pub page_token: Option<String>,
}

impl PaginationParams {
    #[allow(dead_code)]
    pub fn apply(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut req = req;
        if let Some(size) = self.page_size {
            req = req.query(&[("pageSize", size.to_string())]);
        }
        if let Some(ref token) = self.page_token {
            req = req.query(&[("pageToken", token.as_str())]);
        }
        req
    }
}
