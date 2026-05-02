use serde_json::{json, Value};

use crate::error::OdooError;
use crate::odoo_client::OdooClient;

pub async fn create(client: &OdooClient, model: &str, values: Value) -> Result<Value, OdooError> {
    let body = json!({ "vals_list": [values] });
    client.call(model, "create", body).await
}
