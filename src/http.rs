//! MCP streamable HTTP transport: the MCP endpoint on `/mcp` plus a liveness
//! probe on `/health`, so the server can run as a remote tool server (e.g. a
//! compose service next to Open WebUI) instead of a stdio subprocess.

use std::future::IntoFuture;
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::Request;
use axum::http::{header::ORIGIN, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rmcp::transport::streamable_http_server::session::local::{LocalSessionManager, SessionConfig};
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, warn};

use crate::service::OdooService;

/// Largest accepted request body. Tool arguments are small JSON objects; the
/// headroom covers base64 binary field values passed to `create`/`write`.
const MAX_REQUEST_BODY_BYTES: usize = 4 * 1024 * 1024;

/// Deadline for a request to produce its response headers. Tool results are
/// streamed afterwards over SSE and are bounded by the Odoo client timeouts.
const RESPONSE_HEADERS_TIMEOUT: Duration = Duration::from_secs(30);

/// A session without any client message for this long is dropped; the client
/// then re-initializes.
const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// SSE keep-alive ping interval, so proxies do not cut idle streams.
const SSE_KEEP_ALIVE: Duration = Duration::from_secs(15);

/// Time left to open streams to drain after SIGINT/SIGTERM before exiting.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(10);

/// Serve `service` over MCP streamable HTTP on `bind` until SIGINT/SIGTERM.
pub async fn serve(service: OdooService, bind: SocketAddr) -> Result<()> {
    let shutdown = CancellationToken::new();

    let session_manager = LocalSessionManager {
        sessions: Default::default(),
        session_config: SessionConfig {
            channel_capacity: SessionConfig::DEFAULT_CHANNEL_CAPACITY,
            keep_alive: Some(SESSION_IDLE_TIMEOUT),
        },
    };
    let mcp = StreamableHttpService::new(
        move || Ok(service.clone()),
        session_manager.into(),
        StreamableHttpServerConfig {
            sse_keep_alive: Some(SSE_KEEP_ALIVE),
            stateful_mode: true,
            cancellation_token: shutdown.child_token(),
        },
    );

    let app = Router::new()
        .nest_service("/mcp", mcp)
        .layer(middleware::from_fn(reject_browser_origin))
        .route("/health", get(|| async { "ok" }))
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            RESPONSE_HEADERS_TIMEOUT,
        ));

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("Failed to bind {bind}"))?;
    let local_addr = listener
        .local_addr()
        .context("Failed to read the bound address")?;
    info!("MCP streamable HTTP transport listening on http://{local_addr}/mcp");

    let server = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown.clone().cancelled_owned())
        .into_future();

    tokio::select! {
        result = server => result.context("HTTP server failed")?,
        () = async {
            shutdown_signal().await;
            info!("Shutdown requested, draining open streams");
            shutdown.cancel();
            tokio::time::sleep(SHUTDOWN_GRACE).await;
        } => warn!("Streams still open after {SHUTDOWN_GRACE:?}, exiting anyway"),
    }
    Ok(())
}

/// The MCP spec requires validating `Origin` to block DNS rebinding: a web
/// page must not be able to drive a server bound to a private address. MCP
/// clients acting as tool servers are not browsers and send no `Origin`, so
/// any request carrying one is refused.
async fn reject_browser_origin(request: Request, next: Next) -> Response {
    if request.headers().contains_key(ORIGIN) {
        return (
            StatusCode::FORBIDDEN,
            "Forbidden: browser-originated requests are not accepted",
        )
            .into_response();
    }
    next.run(request).await
}

/// Resolves on SIGINT (Ctrl-C) or, on Unix, SIGTERM (`docker stop`). If no
/// handler can be installed it never resolves, so the server keeps running.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            warn!("Cannot listen for Ctrl-C: {e}");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(e) => {
                warn!("Cannot listen for SIGTERM: {e}");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
