use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;

use crate::config::Config;
use crate::error::OdooError;

pub struct OdooClient {
    http: reqwest::Client,
    base_url: String,
}

impl OdooClient {
    pub fn new(config: &Config) -> Result<Self, OdooError> {
        let mut headers = HeaderMap::new();

        let auth_value = HeaderValue::from_str(&format!("Bearer {}", config.odoo_api_key))
            .map_err(|e| OdooError::Auth(format!("invalid API key header: {e}")))?;
        headers.insert(AUTHORIZATION, auth_value);

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let db_value = HeaderValue::from_str(&config.odoo_db)
            .map_err(|e| OdooError::Auth(format!("invalid database header: {e}")))?;
        headers.insert("X-Odoo-Database", db_value);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let base_url = config.odoo_url.as_str().trim_end_matches('/').to_string();

        Ok(Self { http, base_url })
    }

    pub async fn call(&self, model: &str, method: &str, body: Value) -> Result<Value, OdooError> {
        let url = format!("{}/json/2/{}/{}", self.base_url, model, method);

        let response = self.http.post(&url).json(&body).send().await?;

        let status = response.status().as_u16();

        if status == 200 {
            let result: Value = response.json().await?;
            return Ok(result);
        }

        let error_body: Value = response
            .json()
            .await
            .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));

        Err(OdooError::from_http_status(status, &error_body))
    }
}
