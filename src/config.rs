use clap::Parser;
use url::Url;

fn parse_url(s: &str) -> Result<Url, String> {
    Url::parse(s).map_err(|e| format!("invalid URL: {e}"))
}

#[derive(Parser, Debug, Clone)]
#[command(name = "mcp-server-odoo")]
pub struct Config {
    /// URL of the Odoo instance
    #[arg(long, env = "ODOO_URL", value_parser = parse_url)]
    pub odoo_url: Url,

    /// Bearer token for Odoo API authentication
    #[arg(long, env = "ODOO_API_KEY")]
    pub odoo_api_key: String,

    /// Odoo database name
    #[arg(long, env = "ODOO_DB")]
    pub odoo_db: String,

    /// Comma-separated glob patterns for model inclusion (default: *)
    #[arg(long, env = "MODEL_INCLUDE", default_value = "*")]
    model_include_raw: String,

    /// Comma-separated glob patterns for model exclusion
    #[arg(long, env = "MODEL_EXCLUDE", default_value = "")]
    model_exclude_raw: String,

    /// Block all write operations
    #[arg(long, env = "READ_ONLY", default_value_t = false)]
    pub read_only: bool,

    /// Default page size for list operations
    #[arg(long, env = "PAGE_SIZE", default_value_t = 80)]
    pub page_size: u32,
}

impl Config {
    pub fn model_include(&self) -> Vec<String> {
        parse_comma_list(&self.model_include_raw)
    }

    pub fn model_exclude(&self) -> Vec<String> {
        parse_comma_list(&self.model_exclude_raw)
    }
}

fn parse_comma_list(s: &str) -> Vec<String> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',').map(|p| p.trim().to_string()).collect()
}
