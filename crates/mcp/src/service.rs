use crate::tools::AvailableToolsService;
use querying::QueryingService;
use rmcp::model::{
    Implementation, InitializeRequest, InitializeResult, ServerCapabilities, ToolsCapability,
};
use std::sync::Arc;
use tracing::info;

pub trait McpService: Send + Sync {
    fn initialize(&self, request: InitializeRequest) -> InitializeResult;
    fn on_initialized(&self);
    fn list_tools(
        &self,
        request: rmcp::model::PaginatedRequestParam,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::Error>;
    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
    ) -> Result<rmcp::model::CallToolResult, rmcp::Error>;
}

pub struct DefaultMcpService {
    available_tools_service: AvailableToolsService,
}

impl DefaultMcpService {
    pub fn new(query_service: Arc<dyn QueryingService>) -> Self {
        Self {
            available_tools_service: AvailableToolsService::new(query_service),
        }
    }
}

impl McpService for DefaultMcpService {
    fn initialize(&self, request: InitializeRequest) -> InitializeResult {
        InitializeResult {
            protocol_version: request.params.protocol_version,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation::default(),
            instructions: None,
        }
    }

    fn on_initialized(&self) {
        info!("MCP server initialized.");
    }

    fn list_tools(
        &self,
        _request: rmcp::model::PaginatedRequestParam,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::Error> {
        Ok(rmcp::model::ListToolsResult {
            tools: self.available_tools_service.get_available_tools(),
            next_cursor: None,
        })
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
    ) -> Result<rmcp::model::CallToolResult, rmcp::Error> {
        self.available_tools_service
            .call_tool(request.name.as_ref(), request.arguments.unwrap_or_default())
    }
}
