mod config;
mod error;
mod model_filter;
mod odoo_client;
mod service;
mod tools;

use anyhow::{Context, Result};
use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let config = config::Config::parse();
    info!("Connecting to Odoo at {}", config.odoo_url);

    let client = odoo_client::OdooClient::new(&config).context("Failed to create Odoo client")?;

    let service = service::OdooService::new(config, client);

    info!("Starting MCP stdio transport...");
    let server = service
        .serve(stdio())
        .await
        .context("Failed to start MCP server")?;
    server.waiting().await?;

    info!("MCP server stopped");
    Ok(())
}
