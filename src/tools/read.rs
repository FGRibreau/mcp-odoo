use serde_json::{json, Map, Value};

use crate::error::OdooError;
use crate::odoo_client::OdooClient;

pub async fn read(
    client: &OdooClient,
    model: &str,
    ids: Vec<i64>,
    fields: Option<Vec<String>>,
) -> Result<Value, OdooError> {
    let mut body = Map::new();
    body.insert("ids".into(), json!(ids));
    if let Some(f) = fields {
        body.insert("fields".into(), json!(f));
    }
    client.call(model, "read", Value::Object(body)).await
}
