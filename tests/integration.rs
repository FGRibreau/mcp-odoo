//! Black-box integration tests that drive the public library API against a REAL
//! Odoo server — no mocks, no in-process fakes. The exact same suite runs
//! unchanged against Odoo 16 (classic JSON-RPC) and Odoo 19 (JSON/2); only the
//! `ODOO_TEST_*` environment differs, which is precisely what proves the two
//! transports are behaviour-compatible behind `OdooClient::call`.
//!
//! Enable with `--features integration`. Required env: `ODOO_TEST_URL`,
//! `ODOO_TEST_DB`, `ODOO_TEST_API_KEY`. Optional: `ODOO_TEST_LOGIN` (mandatory
//! when the resolved protocol is JSON-RPC) and `ODOO_TEST_PROTOCOL`
//! (auto|json2|jsonrpc, default auto).
#![cfg(feature = "integration")]

use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use serde_json::{json, Map, Value};

use mcp_server_odoo::config::Config;
use mcp_server_odoo::error::OdooError;
use mcp_server_odoo::model_filter::ModelFilter;
use mcp_server_odoo::odoo_client::OdooClient;
use mcp_server_odoo::tools;

fn require_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("integration tests require env var {key}"))
}

// Build a real Config through the public CLI parser, so the test exercises the
// same argument/flag plumbing an operator hits (including ODOO_PROTOCOL/LOGIN).
fn build_config(read_only: bool) -> Config {
    let mut args = vec![
        "mcp-server-odoo".to_string(),
        "--odoo-url".into(),
        require_env("ODOO_TEST_URL"),
        "--odoo-api-key".into(),
        require_env("ODOO_TEST_API_KEY"),
        "--odoo-db".into(),
        require_env("ODOO_TEST_DB"),
        "--odoo-protocol".into(),
        std::env::var("ODOO_TEST_PROTOCOL").unwrap_or_else(|_| "auto".into()),
    ];
    if let Ok(login) = std::env::var("ODOO_TEST_LOGIN") {
        args.push("--odoo-login".into());
        args.push(login);
    }
    if read_only {
        args.push("--read-only".into());
    }
    Config::try_parse_from(args).expect("valid integration config")
}

struct Harness {
    client: OdooClient,
    filter: ModelFilter,
    config: Config,
}

impl Harness {
    async fn connect(read_only: bool) -> Self {
        let config = build_config(read_only);
        let filter = ModelFilter::new(config.model_include(), config.model_exclude());
        let client = OdooClient::new(&config)
            .await
            .expect("connect + authenticate against the live Odoo");
        Self {
            client,
            filter,
            config,
        }
    }

    async fn call(&self, tool: &str, args: Value) -> Result<Value, OdooError> {
        let map: Map<String, Value> = match args {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        tools::dispatch(
            tool,
            map,
            &self.client,
            &self.filter,
            self.config.read_only,
            self.config.page_size,
        )
        .await
    }
}

fn unique_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("mcp-it-{}-{}", std::process::id(), nanos)
}

#[tokio::test]
async fn list_models_discovers_res_partner() {
    let h = Harness::connect(false).await;
    let out = h.call("list_models", json!({})).await.expect("list_models");
    let models = out.as_array().expect("list_models returns an array");
    assert!(!models.is_empty(), "expected at least one model");
    assert!(
        models
            .iter()
            .any(|m| m.get("model").and_then(Value::as_str) == Some("res.partner")),
        "res.partner should be discoverable"
    );
}

#[tokio::test]
async fn describe_model_returns_field_definitions() {
    let h = Harness::connect(false).await;
    let out = h
        .call("describe_model", json!({ "model": "res.partner" }))
        .await
        .expect("describe_model");
    let fields = out.as_object().expect("fields_get returns an object");
    let name = fields
        .get("name")
        .and_then(Value::as_object)
        .expect("res.partner has a 'name' field definition");
    assert!(name.contains_key("type"), "a field def carries its type");
}

#[tokio::test]
async fn search_returns_records_and_count() {
    let h = Harness::connect(false).await;
    let out = h
        .call(
            "search",
            json!({ "model": "res.partner", "domain": [], "limit": 5 }),
        )
        .await
        .expect("search");
    assert!(
        out.get("records").and_then(Value::as_array).is_some(),
        "search yields a records array"
    );
    let total = out
        .get("total")
        .and_then(Value::as_u64)
        .expect("search yields a numeric total (search_count)");
    assert!(total >= 1, "a base install has at least one partner");
    assert!(out.get("has_more").is_some());
    assert!(out.get("next_offset").is_some());
}

#[tokio::test]
async fn read_by_id_returns_the_record() {
    let h = Harness::connect(false).await;
    let found = h
        .call(
            "search",
            json!({ "model": "res.partner", "domain": [], "limit": 1 }),
        )
        .await
        .expect("search for an id");
    let id = found["records"][0]["id"]
        .as_i64()
        .expect("a partner record with an id");

    let read = h
        .call(
            "read",
            json!({ "model": "res.partner", "ids": [id], "fields": ["name"] }),
        )
        .await
        .expect("read");
    let records = read.as_array().expect("read returns an array");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["id"].as_i64(), Some(id));
}

#[tokio::test]
async fn read_only_blocks_mutations_but_allows_reads() {
    let h = Harness::connect(true).await;

    let blocked = [
        (
            "create",
            json!({ "model": "res.partner", "values": { "name": "x" } }),
        ),
        (
            "write",
            json!({ "model": "res.partner", "ids": [1], "values": { "name": "x" } }),
        ),
        ("delete", json!({ "model": "res.partner", "ids": [1] })),
        (
            "call_method",
            json!({ "model": "res.partner", "method": "unlink", "ids": [1] }),
        ),
    ];
    for (tool, args) in blocked {
        match h.call(tool, args).await {
            Err(OdooError::ReadOnly) => {}
            other => panic!("expected ReadOnly for {tool}, got {other:?}"),
        }
    }

    // A read-only method must still go through even under READ_ONLY.
    let count = h
        .call(
            "call_method",
            json!({ "model": "res.partner", "method": "search_count", "kwargs": { "domain": [] } }),
        )
        .await;
    assert!(
        count.is_ok(),
        "search_count via call_method must be allowed in read-only mode: {count:?}"
    );
}

#[tokio::test]
async fn create_write_read_delete_roundtrip() {
    let h = Harness::connect(false).await;
    let name = unique_name();

    h.call(
        "create",
        json!({ "model": "res.partner", "values": { "name": name } }),
    )
    .await
    .expect("create");

    // Resolve the id by the unique name — protocol-agnostic, independent of
    // whatever shape each transport's `create` returns.
    let found = h
        .call(
            "search",
            json!({ "model": "res.partner", "domain": [["name", "=", name]], "fields": ["id", "name"] }),
        )
        .await
        .expect("search created");
    assert_eq!(
        found["total"].as_u64(),
        Some(1),
        "exactly one record created"
    );
    let id = found["records"][0]["id"].as_i64().expect("created id");

    let updated = format!("{name}-updated");
    h.call(
        "write",
        json!({ "model": "res.partner", "ids": [id], "values": { "name": updated } }),
    )
    .await
    .expect("write");

    let read = h
        .call(
            "read",
            json!({ "model": "res.partner", "ids": [id], "fields": ["name"] }),
        )
        .await
        .expect("read back");
    assert_eq!(read[0]["name"].as_str(), Some(updated.as_str()));

    h.call("delete", json!({ "model": "res.partner", "ids": [id] }))
        .await
        .expect("delete");

    let gone = h
        .call(
            "search",
            json!({ "model": "res.partner", "domain": [["id", "=", id]], "fields": ["id"] }),
        )
        .await
        .expect("search after delete");
    assert_eq!(
        gone["total"].as_u64(),
        Some(0),
        "record is gone after delete"
    );
}
