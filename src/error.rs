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

    #[error("Protocol error: {0}")]
    Protocol(String),
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

    /// Map a JSON-RPC `error` object (classic external API) to a variant.
    ///
    /// The classic API returns business failures as HTTP 200 with an `error`
    /// object; the useful Odoo exception class lives in `error.data.name`
    /// (e.g. `odoo.exceptions.AccessError`) and the human-readable text in
    /// `error.data.message`, with `error.message` (e.g. "Odoo Server Error")
    /// as a last-resort fallback. `AccessDenied` means bad credentials, while
    /// `AccessError` means the record/model is forbidden — hence the split.
    pub fn from_jsonrpc_error(error: &Value) -> Self {
        let data = error.get("data");
        let name = data
            .and_then(|d| d.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let message = data
            .and_then(|d| d.get("message"))
            .and_then(Value::as_str)
            .filter(|m| !m.is_empty())
            .or_else(|| error.get("message").and_then(Value::as_str))
            .unwrap_or("unknown error")
            .to_string();

        // The class name is dotted (`odoo.exceptions.AccessError`); match on the
        // trailing segment so a namespace change never silently reclassifies it.
        match name.rsplit('.').next().unwrap_or_default() {
            "AccessDenied" => OdooError::Auth(message),
            "AccessError" => OdooError::AccessDenied(message),
            "MissingError" => OdooError::NotFound(message),
            "ValidationError" | "UserError" => OdooError::Validation(message),
            _ => OdooError::OdooApi {
                name: if name.is_empty() {
                    "odoo.exceptions.ServerError".to_string()
                } else {
                    name.to_string()
                },
                message,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn err(name: &str, message: &str) -> Value {
        json!({
            "code": 200,
            "message": "Odoo Server Error",
            "data": { "name": name, "message": message },
        })
    }

    #[test]
    fn access_denied_maps_to_auth() {
        match OdooError::from_jsonrpc_error(&err("odoo.exceptions.AccessDenied", "wrong")) {
            OdooError::Auth(m) => assert_eq!(m, "wrong"),
            other => panic!("expected Auth, got {other:?}"),
        }
    }

    #[test]
    fn access_error_maps_to_access_denied() {
        match OdooError::from_jsonrpc_error(&err("odoo.exceptions.AccessError", "no rights")) {
            OdooError::AccessDenied(m) => assert_eq!(m, "no rights"),
            other => panic!("expected AccessDenied, got {other:?}"),
        }
    }

    #[test]
    fn missing_error_maps_to_not_found() {
        match OdooError::from_jsonrpc_error(&err("odoo.exceptions.MissingError", "gone")) {
            OdooError::NotFound(m) => assert_eq!(m, "gone"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn validation_and_user_errors_map_to_validation() {
        for name in [
            "odoo.exceptions.ValidationError",
            "odoo.exceptions.UserError",
        ] {
            match OdooError::from_jsonrpc_error(&err(name, "bad")) {
                OdooError::Validation(m) => assert_eq!(m, "bad"),
                other => panic!("expected Validation for {name}, got {other:?}"),
            }
        }
    }

    #[test]
    fn unknown_class_falls_back_to_generic() {
        match OdooError::from_jsonrpc_error(&err("odoo.exceptions.CacheMiss", "oops")) {
            OdooError::OdooApi { name, message } => {
                assert_eq!(name, "odoo.exceptions.CacheMiss");
                assert_eq!(message, "oops");
            }
            other => panic!("expected OdooApi, got {other:?}"),
        }
    }

    #[test]
    fn missing_data_falls_back_to_top_level_message() {
        let raw = json!({ "code": 200, "message": "Odoo Server Error" });
        match OdooError::from_jsonrpc_error(&raw) {
            OdooError::OdooApi { name, message } => {
                assert_eq!(name, "odoo.exceptions.ServerError");
                assert_eq!(message, "Odoo Server Error");
            }
            other => panic!("expected OdooApi, got {other:?}"),
        }
    }

    proptest::proptest! {
        // Any server-supplied `error` object must classify without panicking.
        #[test]
        fn never_panics_on_arbitrary_json(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..512)) {
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                let _ = OdooError::from_jsonrpc_error(&value);
            }
        }
    }
}
