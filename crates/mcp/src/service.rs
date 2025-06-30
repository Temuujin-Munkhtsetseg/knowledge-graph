use crate::tools::{call::handle_tool_call_internal, list::get_available_tools};
use rmcp::model::{
    Implementation, InitializeRequest, InitializeResult, ServerCapabilities, ToolsCapability,
};
use tracing::info;

#[derive(Clone, Default)]
pub struct McpService;

impl McpService {
    pub fn initialize(&self, request: InitializeRequest) -> InitializeResult {
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

    pub fn on_initialized(&self) {
        info!("MCP server initialized.");
    }

    pub fn list_tools(
        &self,
        _request: rmcp::model::PaginatedRequestParam,
    ) -> Result<rmcp::model::ListToolsResult, rmcp::Error> {
        Ok(rmcp::model::ListToolsResult {
            tools: get_available_tools(),
            next_cursor: None,
        })
    }

    pub fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
    ) -> Result<rmcp::model::CallToolResult, rmcp::Error> {
        Ok(handle_tool_call_internal(request))
    }
}
