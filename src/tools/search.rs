use serde_json::{json, Map, Value};

use crate::error::OdooError;
use crate::odoo_client::OdooClient;

#[allow(clippy::too_many_arguments)]
pub async fn search(
    client: &OdooClient,
    model: &str,
    domain: Value,
    fields: Option<Vec<String>>,
    limit: Option<u32>,
    offset: Option<u32>,
    order: Option<String>,
    default_page_size: u32,
) -> Result<Value, OdooError> {
    let effective_limit = limit.unwrap_or(default_page_size);
    let effective_offset = offset.unwrap_or(0);

    let mut search_body = Map::new();
    search_body.insert("domain".into(), domain.clone());
    search_body.insert(
        "fields".into(),
        match &fields {
            Some(f) => json!(f),
            None => json!([]),
        },
    );
    search_body.insert("limit".into(), json!(effective_limit));
    search_body.insert("offset".into(), json!(effective_offset));
    if let Some(ref o) = order {
        search_body.insert("order".into(), json!(o));
    }

    let count_body = json!({ "domain": domain });

    let (records_result, count_result) = tokio::join!(
        client.call(model, "search_read", Value::Object(search_body)),
        client.call(model, "search_count", count_body),
    );

    let records = records_result?;
    let total = count_result?;

    let total_num = total.as_u64().unwrap_or(0);
    let next_offset = effective_offset as u64 + effective_limit as u64;
    let has_more = next_offset < total_num;

    Ok(json!({
        "records": records,
        "total": total,
        "has_more": has_more,
        "next_offset": next_offset,
    }))
}
