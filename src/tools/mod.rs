mod call_method;
mod create;
mod delete;
mod describe_model;
mod list_models;
mod read;
mod search;
mod write;

use serde_json::{json, Map, Value};

use crate::error::OdooError;
use crate::model_filter::ModelFilter;
use crate::odoo_client::OdooClient;

pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: Map<String, Value>,
}

pub fn tool_schemas() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "list_models",
            description: "List all Odoo models available (filtered by server configuration)",
            schema: to_map(json!({
                "type": "object",
                "properties": {},
                "required": [],
            })),
        },
        ToolDef {
            name: "describe_model",
            description: "Get field definitions for an Odoo model",
            schema: to_map(json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "Technical model name (e.g. res.partner)" }
                },
                "required": ["model"],
            })),
        },
        ToolDef {
            name: "search",
            description: "Search records in an Odoo model with domain filters. Returns paginated results with total count.",
            schema: to_map(json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "Technical model name (e.g. res.partner)" },
                    "domain": { "type": "array", "description": "Odoo domain filter (e.g. [[\"is_company\",\"=\",true]])" },
                    "fields": { "type": "array", "items": { "type": "string" }, "description": "Fields to return" },
                    "limit": { "type": "integer", "description": "Maximum number of records" },
                    "offset": { "type": "integer", "description": "Number of records to skip" },
                    "order": { "type": "string", "description": "Sort order (e.g. \"name asc, id desc\")" }
                },
                "required": ["model", "domain"],
            })),
        },
        ToolDef {
            name: "read",
            description: "Read specific records by their IDs",
            schema: to_map(json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "Technical model name (e.g. res.partner)" },
                    "ids": { "type": "array", "items": { "type": "integer" }, "description": "Record IDs to read" },
                    "fields": { "type": "array", "items": { "type": "string" }, "description": "Fields to return" }
                },
                "required": ["model", "ids"],
            })),
        },
        ToolDef {
            name: "create",
            description: "Create a new record in an Odoo model",
            schema: to_map(json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "Technical model name (e.g. res.partner)" },
                    "values": { "type": "object", "description": "Field values for the new record" }
                },
                "required": ["model", "values"],
            })),
        },
        ToolDef {
            name: "write",
            description: "Update existing records in an Odoo model",
            schema: to_map(json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "Technical model name (e.g. res.partner)" },
                    "ids": { "type": "array", "items": { "type": "integer" }, "description": "Record IDs to update" },
                    "values": { "type": "object", "description": "Field values to update" }
                },
                "required": ["model", "ids", "values"],
            })),
        },
        ToolDef {
            name: "delete",
            description: "Delete records from an Odoo model",
            schema: to_map(json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "Technical model name (e.g. res.partner)" },
                    "ids": { "type": "array", "items": { "type": "integer" }, "description": "Record IDs to delete" }
                },
                "required": ["model", "ids"],
            })),
        },
        ToolDef {
            name: "call_method",
            description: "Call an arbitrary method on an Odoo model",
            schema: to_map(json!({
                "type": "object",
                "properties": {
                    "model": { "type": "string", "description": "Technical model name (e.g. res.partner)" },
                    "method": { "type": "string", "description": "Method name to call" },
                    "ids": { "type": "array", "items": { "type": "integer" }, "description": "Record IDs (optional for @api.model methods)" },
                    "kwargs": { "type": "object", "description": "Keyword arguments to pass to the method" }
                },
                "required": ["model", "method"],
            })),
        },
    ]
}

fn to_map(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

pub async fn dispatch(
    tool_name: &str,
    args: Map<String, Value>,
    client: &OdooClient,
    filter: &ModelFilter,
    read_only: bool,
    page_size: u32,
) -> Result<Value, OdooError> {
    match tool_name {
        "list_models" => list_models::list_models(client, filter).await,

        "describe_model" => {
            let model = require_str(&args, "model")?;
            describe_model::describe_model(client, &model).await
        }

        "search" => {
            let model = require_str(&args, "model")?;
            let domain = require_value(&args, "domain")?;
            let fields = optional_string_array(&args, "fields");
            let limit = optional_u32(&args, "limit");
            let offset = optional_u32(&args, "offset");
            let order = optional_str(&args, "order");
            search::search(
                client, &model, domain, fields, limit, offset, order, page_size,
            )
            .await
        }

        "read" => {
            let model = require_str(&args, "model")?;
            let ids = require_i64_array(&args, "ids")?;
            let fields = optional_string_array(&args, "fields");
            read::read(client, &model, ids, fields).await
        }

        "create" => {
            guard_write(read_only)?;
            let model = require_str(&args, "model")?;
            let values = require_value(&args, "values")?;
            create::create(client, &model, values).await
        }

        "write" => {
            guard_write(read_only)?;
            let model = require_str(&args, "model")?;
            let ids = require_i64_array(&args, "ids")?;
            let values = require_value(&args, "values")?;
            write::write(client, &model, ids, values).await
        }

        "delete" => {
            guard_write(read_only)?;
            let model = require_str(&args, "model")?;
            let ids = require_i64_array(&args, "ids")?;
            delete::delete(client, &model, ids).await
        }

        "call_method" => {
            let model = require_str(&args, "model")?;
            let method = require_str(&args, "method")?;
            if read_only && !is_read_only_method(&method) {
                return Err(OdooError::ReadOnly);
            }
            let ids = optional_i64_array(&args, "ids");
            let kwargs = args.get("kwargs").and_then(Value::as_object).cloned();
            call_method::call_method(client, &model, &method, ids, kwargs).await
        }

        _ => Err(OdooError::Validation(format!("unknown tool: {tool_name}"))),
    }
}

fn guard_write(read_only: bool) -> Result<(), OdooError> {
    if read_only {
        return Err(OdooError::ReadOnly);
    }
    Ok(())
}

fn is_read_only_method(method: &str) -> bool {
    matches!(
        method,
        "name_get"
            | "name_search"
            | "read_group"
            | "fields_get"
            | "search"
            | "search_read"
            | "search_count"
            | "default_get"
    ) || method.starts_with("get_")
        || method.starts_with("check_")
        || method.starts_with("has_")
        || method.starts_with("is_")
        || method.starts_with("can_")
}

fn require_str(args: &Map<String, Value>, key: &str) -> Result<String, OdooError> {
    args.get(key)
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| OdooError::Validation(format!("missing required string parameter: {key}")))
}

fn require_value(args: &Map<String, Value>, key: &str) -> Result<Value, OdooError> {
    args.get(key)
        .cloned()
        .ok_or_else(|| OdooError::Validation(format!("missing required parameter: {key}")))
}

fn require_i64_array(args: &Map<String, Value>, key: &str) -> Result<Vec<i64>, OdooError> {
    let arr = args
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| OdooError::Validation(format!("missing required array parameter: {key}")))?;

    arr.iter()
        .map(|v| {
            v.as_i64()
                .ok_or_else(|| OdooError::Validation(format!("non-integer value in {key} array")))
        })
        .collect()
}

fn optional_str(args: &Map<String, Value>, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(String::from)
}

fn optional_u32(args: &Map<String, Value>, key: &str) -> Option<u32> {
    args.get(key).and_then(Value::as_u64).map(|v| v as u32)
}

fn optional_string_array(args: &Map<String, Value>, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect()
    })
}

fn optional_i64_array(args: &Map<String, Value>, key: &str) -> Option<Vec<i64>> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_i64).collect())
}
