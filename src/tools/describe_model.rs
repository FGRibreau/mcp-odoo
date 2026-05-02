use serde_json::json;

use crate::error::OdooError;
use crate::odoo_client::OdooClient;

pub async fn describe_model(
    client: &OdooClient,
    model: &str,
) -> Result<serde_json::Value, OdooError> {
    client.call(model, "fields_get", json!({})).await
}
