use serde_json::{json, Value};

use crate::error::OdooError;
use crate::odoo_client::OdooClient;

pub async fn delete(client: &OdooClient, model: &str, ids: Vec<i64>) -> Result<Value, OdooError> {
    let body = json!({ "ids": ids });
    client.call(model, "unlink", body).await
}
