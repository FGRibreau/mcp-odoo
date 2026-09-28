//! Odoo client with two interchangeable transports behind a single `call` entry
//! point, so the tools layer never learns which protocol is in use.
//!
//! Odoo 19 introduced the JSON/2 API (`POST /json/2/{model}/{method}`, a Bearer
//! API key plus an `X-Odoo-Database` header, one HTTP call per ORM method with
//! the arguments passed as a named-args object). Odoo <= 18 only ships the
//! classic external API (`POST /jsonrpc`): a stateless `authenticate` returns a
//! numeric uid, then every ORM method goes through `object.execute_kw` with
//! positional `args` and keyword `kwargs`.
//!
//! Auto-detection: `POST /web/webclient/version_info` is an unauthenticated
//! JSON-RPC endpoint present on both eras; its `server_version_info[0]` major
//! number selects the transport (>= 19 -> JSON/2, else JSON-RPC). This keeps a
//! single `ODOO_URL` working across versions without the operator guessing.
//!
//! Argument mapping (JSON/2 named-args object -> execute_kw `args`/`kwargs`):
//! every tool builds a JSON/2 body, and `map_body_to_args` translates it once
//! for the JSON-RPC path. `create` is special-cased because JSON/2 names its
//! payload `vals_list` while Odoo <= 18 takes it as the first *positional*
//! argument of `create` (passing it as a keyword breaks older `create(self,
//! vals)` overrides). A body carrying `ids` targets a concrete recordset, so
//! `ids` becomes `args[0]` (`read`, `write`, `unlink`, record-bound
//! `call_method`); otherwise every entry is a keyword argument (`search_read`,
//! `search_count`, `fields_get`, `@api.model` methods).

use std::time::Duration;

use reqwest::header::{HeaderValue, AUTHORIZATION};
use serde_json::{json, Map, Value};

use crate::config::{Config, Protocol};
use crate::error::OdooError;

// Network bounds: a slow TLS handshake is a fast failure, but ORM methods over
// large datasets legitimately take a while, hence the wider request deadline.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

// Odoo 19 is the first release exposing the JSON/2 API.
const FIRST_JSON2_MAJOR: u64 = 19;

/// The resolved transport, carrying exactly the state each protocol needs.
enum Transport {
    /// Pre-built request headers for the JSON/2 API (validated once at startup).
    Json2 {
        authorization: HeaderValue,
        database: HeaderValue,
    },
    /// Classic external API state; `uid` is resolved once via `authenticate`.
    Jsonrpc {
        database: String,
        api_key: String,
        uid: i64,
    },
}

pub struct OdooClient {
    http: reqwest::Client,
    base_url: String,
    transport: Transport,
}

impl OdooClient {
    /// Build the client, resolving the transport (and, for JSON-RPC, the uid)
    /// with one or two startup HTTP calls. Fails fast on an unreachable server,
    /// a rejected login, or a JSON-RPC protocol selected without `ODOO_LOGIN`.
    pub async fn new(config: &Config) -> Result<Self, OdooError> {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        let base_url = config.odoo_url.as_str().trim_end_matches('/').to_string();

        let use_json2 = match config.odoo_protocol {
            Protocol::Json2 => true,
            Protocol::Jsonrpc => false,
            Protocol::Auto => detect_major_version(&http, &base_url).await? >= FIRST_JSON2_MAJOR,
        };

        let transport = if use_json2 {
            let authorization =
                HeaderValue::from_str(&format!("Bearer {}", config.odoo_api_key))
                    .map_err(|e| OdooError::Auth(format!("invalid API key header: {e}")))?;
            let database = HeaderValue::from_str(&config.odoo_db)
                .map_err(|e| OdooError::Auth(format!("invalid database header: {e}")))?;
            Transport::Json2 {
                authorization,
                database,
            }
        } else {
            let login = config.odoo_login.as_deref().ok_or_else(|| {
                OdooError::Protocol(
                    "ODOO_LOGIN is required for the JSON-RPC transport (Odoo <= 18); \
                     set it to the user login/email"
                        .to_string(),
                )
            })?;
            let uid = authenticate(
                &http,
                &base_url,
                &config.odoo_db,
                login,
                &config.odoo_api_key,
            )
            .await?;
            Transport::Jsonrpc {
                database: config.odoo_db.clone(),
                api_key: config.odoo_api_key.clone(),
                uid,
            }
        };

        Ok(Self {
            http,
            base_url,
            transport,
        })
    }

    /// Invoke `method` on `model`, passing the JSON/2 named-args `body`. The
    /// return value has the same shape on both transports so tools stay
    /// protocol-agnostic.
    pub async fn call(&self, model: &str, method: &str, body: Value) -> Result<Value, OdooError> {
        match &self.transport {
            Transport::Json2 {
                authorization,
                database,
            } => {
                self.call_json2(authorization, database, model, method, body)
                    .await
            }
            Transport::Jsonrpc {
                database,
                api_key,
                uid,
            } => {
                self.call_jsonrpc(database, api_key, *uid, model, method, body)
                    .await
            }
        }
    }

    async fn call_json2(
        &self,
        authorization: &HeaderValue,
        database: &HeaderValue,
        model: &str,
        method: &str,
        body: Value,
    ) -> Result<Value, OdooError> {
        let url = format!("{}/json/2/{}/{}", self.base_url, model, method);

        let response = self
            .http
            .post(&url)
            .header(AUTHORIZATION, authorization)
            .header("X-Odoo-Database", database)
            .json(&body)
            .send()
            .await?;

        let status = response.status().as_u16();
        if status == 200 {
            return Ok(response.json().await?);
        }

        let error_body: Value = response
            .json()
            .await
            .unwrap_or_else(|_| Value::Object(Map::new()));

        Err(OdooError::from_http_status(status, &error_body))
    }

    async fn call_jsonrpc(
        &self,
        database: &str,
        api_key: &str,
        uid: i64,
        model: &str,
        method: &str,
        body: Value,
    ) -> Result<Value, OdooError> {
        let (args, kwargs) = map_body_to_args(method, body);

        let envelope = json!({
            "jsonrpc": "2.0",
            "method": "call",
            "params": {
                "service": "object",
                "method": "execute_kw",
                "args": [database, uid, api_key, model, method, args, kwargs],
            },
        });

        let value = self.post_jsonrpc(&envelope).await?;
        Ok(value.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Send a JSON-RPC envelope and return the decoded response, mapping an
    /// `error` object (HTTP 200 with a business failure) to an `OdooError`.
    async fn post_jsonrpc(&self, envelope: &Value) -> Result<Value, OdooError> {
        let url = format!("{}/jsonrpc", self.base_url);
        let response: Value = self
            .http
            .post(&url)
            .json(envelope)
            .send()
            .await?
            .json()
            .await?;
        if let Some(error) = response.get("error") {
            return Err(OdooError::from_jsonrpc_error(error));
        }
        Ok(response)
    }
}

/// Translate a JSON/2 named-args object into `execute_kw`'s positional `args`
/// list and keyword `kwargs` object. See the module design comment for the rule.
fn map_body_to_args(method: &str, body: Value) -> (Value, Value) {
    let mut obj = match body {
        Value::Object(map) => map,
        // A non-object body has no named args; forward it as a lone positional
        // so odd callers degrade predictably instead of silently dropping data.
        other => return (json!([other]), json!({})),
    };

    if method == "create" {
        if let Some(vals_list) = obj.remove("vals_list") {
            return (json!([vals_list]), Value::Object(obj));
        }
    }

    if let Some(ids) = obj.remove("ids") {
        return (json!([ids]), Value::Object(obj));
    }

    (json!([]), Value::Object(obj))
}

/// Probe the unauthenticated `version_info` endpoint and return the major
/// version (`server_version_info[0]`).
async fn detect_major_version(http: &reqwest::Client, base_url: &str) -> Result<u64, OdooError> {
    let url = format!("{base_url}/web/webclient/version_info");
    let envelope = json!({ "jsonrpc": "2.0", "method": "call", "params": {} });

    let response: Value = http.post(&url).json(&envelope).send().await?.json().await?;
    if let Some(error) = response.get("error") {
        return Err(OdooError::from_jsonrpc_error(error));
    }

    response
        .get("result")
        .and_then(|r| r.get("server_version_info"))
        .and_then(Value::as_array)
        .and_then(|parts| parts.first())
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            OdooError::Protocol(
                "version_info did not return a numeric server_version_info[0]; \
                 set ODOO_PROTOCOL explicitly"
                    .to_string(),
            )
        })
}

/// Authenticate against the classic external API and return the uid. Odoo
/// returns `false` (not an error) for bad credentials, which maps to `Auth`.
async fn authenticate(
    http: &reqwest::Client,
    base_url: &str,
    database: &str,
    login: &str,
    api_key: &str,
) -> Result<i64, OdooError> {
    let url = format!("{base_url}/jsonrpc");
    let envelope = json!({
        "jsonrpc": "2.0",
        "method": "call",
        "params": {
            "service": "common",
            "method": "authenticate",
            "args": [database, login, api_key, {}],
        },
    });

    let response: Value = http.post(&url).json(&envelope).send().await?.json().await?;
    if let Some(error) = response.get("error") {
        return Err(OdooError::from_jsonrpc_error(error));
    }

    match response.get("result").and_then(Value::as_i64) {
        Some(uid) if uid > 0 => Ok(uid),
        _ => Err(OdooError::Auth(
            "Odoo rejected the login/API key (authenticate returned no uid)".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lifts_vals_list_to_first_positional() {
        let (args, kwargs) = map_body_to_args("create", json!({ "vals_list": [{ "name": "x" }] }));
        assert_eq!(args, json!([[{ "name": "x" }]]));
        assert_eq!(kwargs, json!({}));
    }

    #[test]
    fn ids_becomes_recordset_positional() {
        let (args, kwargs) = map_body_to_args(
            "write",
            json!({ "ids": [1, 2], "vals": { "active": false } }),
        );
        assert_eq!(args, json!([[1, 2]]));
        assert_eq!(kwargs, json!({ "vals": { "active": false } }));
    }

    #[test]
    fn read_keeps_fields_as_kwargs() {
        let (args, kwargs) = map_body_to_args("read", json!({ "ids": [7], "fields": ["name"] }));
        assert_eq!(args, json!([[7]]));
        assert_eq!(kwargs, json!({ "fields": ["name"] }));
    }

    #[test]
    fn unlink_has_only_the_recordset() {
        let (args, kwargs) = map_body_to_args("unlink", json!({ "ids": [3] }));
        assert_eq!(args, json!([[3]]));
        assert_eq!(kwargs, json!({}));
    }

    #[test]
    fn model_methods_pass_everything_as_kwargs() {
        let (args, kwargs) = map_body_to_args(
            "search_read",
            json!({ "domain": [], "fields": ["name"], "limit": 10 }),
        );
        assert_eq!(args, json!([]));
        assert_eq!(
            kwargs,
            json!({ "domain": [], "fields": ["name"], "limit": 10 })
        );
    }

    #[test]
    fn empty_body_is_no_args() {
        let (args, kwargs) = map_body_to_args("fields_get", json!({}));
        assert_eq!(args, json!([]));
        assert_eq!(kwargs, json!({}));
    }

    #[test]
    fn non_object_body_becomes_lone_positional() {
        let (args, kwargs) = map_body_to_args("some_method", json!(42));
        assert_eq!(args, json!([42]));
        assert_eq!(kwargs, json!({}));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // A body built from a vocabulary that deliberately includes the two keys
    // with positional semantics (`ids`, `vals_list`), so the invariants below
    // are actually exercised rather than always hitting the kwargs-only branch.
    fn method_and_body() -> impl Strategy<Value = (String, Map<String, Value>)> {
        let method = prop_oneof![
            Just("create".to_string()),
            Just("write".to_string()),
            Just("read".to_string()),
            Just("unlink".to_string()),
            Just("search_read".to_string()),
            Just("message_post".to_string()),
        ];
        let key = prop_oneof![
            Just("ids".to_string()),
            Just("vals_list".to_string()),
            Just("vals".to_string()),
            Just("domain".to_string()),
            Just("fields".to_string()),
            Just("body".to_string()),
        ];
        let entries = prop::collection::vec((key, any::<i64>()), 0..6);
        (method, entries).prop_map(|(method, kvs)| {
            let mut map = Map::new();
            for (k, v) in kvs {
                map.insert(k, json!(v));
            }
            (method, map)
        })
    }

    proptest! {
        // The mapping lifts at most one key to a positional argument, and every
        // other entry survives untouched as a keyword argument — no data is
        // dropped or duplicated whatever the body shape.
        #[test]
        fn lifts_exactly_one_key_and_preserves_the_rest((method, body_map) in method_and_body()) {
            let (args, kwargs) = map_body_to_args(&method, Value::Object(body_map.clone()));

            let args_arr = args.as_array().expect("args is always an array");
            prop_assert!(args_arr.len() <= 1);

            let lifted = if method == "create" && body_map.contains_key("vals_list") {
                Some("vals_list")
            } else if body_map.contains_key("ids") {
                Some("ids")
            } else {
                None
            };

            match lifted {
                Some(key) => {
                    prop_assert_eq!(args_arr.len(), 1);
                    prop_assert_eq!(&args_arr[0], &body_map[key]);
                    let mut expected = body_map.clone();
                    expected.remove(key);
                    prop_assert_eq!(kwargs, Value::Object(expected));
                }
                None => {
                    prop_assert!(args_arr.is_empty());
                    prop_assert_eq!(kwargs, Value::Object(body_map));
                }
            }
        }

        // A server response is untrusted input; decoding then mapping arbitrary
        // bytes must never panic (a panic would take down the MCP client).
        #[test]
        fn never_panics_on_arbitrary_json(
            bytes in prop::collection::vec(any::<u8>(), 0..512),
            method in "[a-z_]{0,24}",
        ) {
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes) {
                let _ = map_body_to_args(&method, value);
            }
        }
    }
}
