use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData;
use std::borrow::Cow;
use std::sync::Arc;

use crate::config::Config;
use crate::model_filter::ModelFilter;
use crate::odoo_client::OdooClient;
use crate::tools;

#[derive(Clone)]
pub struct OdooService {
    client: Arc<OdooClient>,
    filter: Arc<ModelFilter>,
    config: Arc<Config>,
}

impl OdooService {
    pub fn new(config: Config, client: OdooClient) -> Self {
        let filter = ModelFilter::new(config.model_include(), config.model_exclude());
        Self {
            client: Arc::new(client),
            filter: Arc::new(filter),
            config: Arc::new(config),
        }
    }
}

impl ServerHandler for OdooService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mcp-server-odoo".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                title: None,
                website_url: None,
            },
            instructions: Some(
                "Odoo MCP server providing tools to interact with an Odoo instance via JSON/2 API."
                    .to_string(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let tools = tools::tool_schemas()
            .into_iter()
            .map(|def| Tool {
                name: Cow::Owned(def.name.to_string()),
                description: Some(Cow::Owned(def.description.to_string())),
                input_schema: Arc::new(def.schema),
                annotations: None,
                icons: None,
                meta: None,
                output_schema: None,
                title: None,
            })
            .collect();

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_name = request.name.as_ref();
        let args: serde_json::Map<String, serde_json::Value> = match request.arguments {
            Some(map) => map.into_iter().collect(),
            None => serde_json::Map::new(),
        };

        match tools::dispatch(
            tool_name,
            args,
            &self.client,
            &self.filter,
            self.config.read_only,
            self.config.page_size,
        )
        .await
        {
            Ok(value) => {
                let text = serde_json::to_string_pretty(&value)
                    .unwrap_or_else(|e| format!("Failed to serialize result: {e}"));
                Ok(CallToolResult {
                    content: vec![Content::text(text)],
                    is_error: Some(false),
                    meta: None,
                    structured_content: None,
                })
            }
            Err(e) => Ok(CallToolResult {
                content: vec![Content::text(format!("Error: {e}"))],
                is_error: Some(true),
                meta: None,
                structured_content: None,
            }),
        }
    }
}
