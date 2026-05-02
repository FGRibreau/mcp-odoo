use serde_json::json;

use crate::error::OdooError;
use crate::model_filter::ModelFilter;
use crate::odoo_client::OdooClient;

pub async fn list_models(
    client: &OdooClient,
    filter: &ModelFilter,
) -> Result<serde_json::Value, OdooError> {
    let body = json!({
        "domain": [],
        "fields": ["model", "name", "info"]
    });
    let result = client.call("ir.model", "search_read", body).await?;
    let models = match result {
        serde_json::Value::Array(arr) => arr,
        other => return Ok(other),
    };
    let filtered = filter.filter(models);
    Ok(serde_json::Value::Array(filtered))
}
