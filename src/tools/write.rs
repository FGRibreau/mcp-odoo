use serde_json::{json, Value};

use crate::error::OdooError;
use crate::odoo_client::OdooClient;

pub async fn write(
    client: &OdooClient,
    model: &str,
    ids: Vec<i64>,
    values: Value,
) -> Result<Value, OdooError> {
    let body = json!({
        "ids": ids,
        "vals": values,
    });
    client.call(model, "write", body).await
}
