use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum OdooError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Access denied: {0}")]
    AccessDenied(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Odoo API error ({name}): {message}")]
    OdooApi { name: String, message: String },

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Read-only mode: write operations are blocked")]
    ReadOnly,
}

impl OdooError {
    pub fn from_http_status(status: u16, body: &Value) -> Self {
        let default_message = body
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error")
            .to_string();

        match status {
            401 => OdooError::Auth(default_message),
            403 => OdooError::AccessDenied(default_message),
            404 => OdooError::NotFound(default_message),
            422 => OdooError::Validation(default_message),
            500 => {
                let name = body
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("odoo.exceptions.ServerError")
                    .to_string();
                let message = body
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Internal Server Error")
                    .to_string();
                OdooError::OdooApi { name, message }
            }
            _ => OdooError::OdooApi {
                name: format!("HTTP {status}"),
                message: default_message,
            },
        }
    }
}
