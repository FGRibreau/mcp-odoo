use serde_json::{json, Map, Value};

use crate::error::OdooError;
use crate::odoo_client::OdooClient;

pub async fn call_method(
    client: &OdooClient,
    model: &str,
    method: &str,
    ids: Option<Vec<i64>>,
    kwargs: Option<Map<String, Value>>,
) -> Result<Value, OdooError> {
    let mut body = kwargs.unwrap_or_default();
    if let Some(ids) = ids {
        body.insert("ids".into(), json!(ids));
    }
    client.call(model, method, Value::Object(body)).await
}
