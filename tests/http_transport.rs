//! Black-box tests of the streamable HTTP transport: they launch the real
//! binary with `--transport http` and speak MCP to it over HTTP, no mocks.
//!
//! No Odoo server is needed: `--odoo-protocol json2` skips the startup version
//! probe, and `initialize` / `tools/list` never call Odoo. The Odoo URL points
//! at a closed port so any unexpected Odoo call would fail loudly.

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{json, Value};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const ACCEPT_MCP: &str = "application/json, text/event-stream";
const LISTENING_PREFIX: &str = "listening on http://";

/// The spawned server, killed when dropped so a failing test never leaks it.
struct Server {
    child: Child,
    base_url: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_server() -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mcp-server-odoo"))
        .args([
            "--transport",
            "http",
            "--bind",
            "127.0.0.1:0",
            "--odoo-url",
            "http://127.0.0.1:9",
            "--odoo-api-key",
            "unused",
            "--odoo-db",
            "unused",
            "--odoo-protocol",
            "json2",
        ])
        .env("RUST_LOG", "info")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mcp-server-odoo");

    let stderr = child.stderr.take().expect("piped stderr");
    let (tx, rx) = mpsc::channel();
    // Keep draining stderr for the server's whole life: closing the pipe would
    // make its log writes fail.
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });

    // `--bind 127.0.0.1:0` lets the OS pick a free port; the server logs the
    // address it actually bound, which is read back here.
    let deadline = std::time::Instant::now() + STARTUP_TIMEOUT;
    let mut seen = Vec::new();
    let base_url = loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let line = rx
            .recv_timeout(remaining)
            .unwrap_or_else(|e| panic!("server did not start ({e}); stderr so far: {seen:#?}"));
        if let Some(start) = line.find(LISTENING_PREFIX) {
            let addr = &line[start + LISTENING_PREFIX.len()..];
            let addr = addr.split('/').next().expect("address before /mcp");
            break format!("http://{addr}");
        }
        seen.push(line);
    };

    Server { child, base_url }
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .expect("build reqwest client")
}

/// Read an SSE response until its first `data:` event and parse it as JSON.
async fn first_sse_event(mut response: reqwest::Response) -> Value {
    let mut buffer = String::new();
    while let Some(chunk) = response.chunk().await.expect("read SSE chunk") {
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        let data = buffer
            .lines()
            .find_map(|l| l.strip_prefix("data:"))
            .map(str::trim);
        if let Some(Ok(event)) = data.map(serde_json::from_str::<Value>) {
            return event;
        }
    }
    panic!("SSE stream ended without a JSON event; received: {buffer:?}");
}

async fn post_mcp(
    client: &reqwest::Client,
    server: &Server,
    session: Option<&str>,
    body: &Value,
) -> reqwest::Response {
    let mut request = client
        .post(format!("{}/mcp", server.base_url))
        .header("Accept", ACCEPT_MCP)
        .json(body);
    if let Some(id) = session {
        request = request.header("Mcp-Session-Id", id);
    }
    request.send().await.expect("POST /mcp")
}

#[tokio::test]
async fn initialize_then_tools_list_over_http() {
    let server = spawn_server();
    let client = http_client();

    let init = post_mcp(
        &client,
        &server,
        None,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": { "name": "http-transport-test", "version": "0" }
            }
        }),
    )
    .await;
    assert_eq!(init.status(), 200);
    let session = init
        .headers()
        .get("mcp-session-id")
        .expect("initialize must return Mcp-Session-Id")
        .to_str()
        .expect("ASCII session id")
        .to_string();
    let init_event = first_sse_event(init).await;
    assert_eq!(init_event["id"], 1);
    assert_eq!(
        init_event["result"]["serverInfo"]["name"],
        "mcp-server-odoo"
    );
    assert!(init_event["result"]["capabilities"]["tools"].is_object());

    let initialized = post_mcp(
        &client,
        &server,
        Some(&session),
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    )
    .await;
    assert_eq!(initialized.status(), 202);

    let list = post_mcp(
        &client,
        &server,
        Some(&session),
        &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
    )
    .await;
    assert_eq!(list.status(), 200);
    let list_event = first_sse_event(list).await;
    assert_eq!(list_event["id"], 2);
    let mut listed: Vec<&str> = list_event["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("tool name"))
        .collect();
    let mut expected: Vec<&str> = mcp_server_odoo::tools::tool_schemas()
        .iter()
        .map(|t| t.name)
        .collect();
    listed.sort_unstable();
    expected.sort_unstable();
    assert_eq!(listed, expected);
}

#[tokio::test]
async fn health_endpoint_answers_ok() {
    let server = spawn_server();
    let response = http_client()
        .get(format!("{}/health", server.base_url))
        .send()
        .await
        .expect("GET /health");
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.expect("health body"), "ok");
}

#[tokio::test]
async fn browser_origin_is_rejected() {
    let server = spawn_server();
    let response = http_client()
        .post(format!("{}/mcp", server.base_url))
        .header("Accept", ACCEPT_MCP)
        .header("Origin", "http://attacker.example")
        .json(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }))
        .send()
        .await
        .expect("POST /mcp with Origin");
    assert_eq!(response.status(), 403);
}

/// Announces a body above the limit without sending it: the server must refuse
/// from `Content-Length` alone, before buffering anything. A raw socket is used
/// because an HTTP client streaming the body races the early 413 and the reset.
#[test]
fn oversized_body_is_rejected() {
    use std::io::{Read, Write};

    let server = spawn_server();
    let addr = server.base_url.trim_start_matches("http://");
    let mut stream = std::net::TcpStream::connect(addr).expect("connect");
    stream
        .set_read_timeout(Some(HTTP_TIMEOUT))
        .expect("set read timeout");
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {addr}\r\nAccept: {ACCEPT_MCP}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        5 * 1024 * 1024
    );
    stream.write_all(request.as_bytes()).expect("send headers");
    let mut status_line = [0u8; 12];
    stream
        .read_exact(&mut status_line)
        .expect("read status line");
    assert_eq!(&status_line, b"HTTP/1.1 413");
}
