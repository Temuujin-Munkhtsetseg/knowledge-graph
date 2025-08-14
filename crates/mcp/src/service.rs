use crate::tools::AvailableToolsService;
use database::kuzu::database::KuzuDatabase;
use database::querying::types::QueryingService;
use event_bus::EventBus;
use rmcp::model::{
    Implementation, InitializeRequestParam, InitializeResult, ServerCapabilities, ToolsCapability,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use std::sync::Arc;
use workspace_manager::WorkspaceManager;

pub struct DefaultMcpService {
    available_tools_service: AvailableToolsService,
}

impl DefaultMcpService {
    pub fn new(
        query_service: Arc<dyn QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
        database: Arc<KuzuDatabase>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            available_tools_service: AvailableToolsService::new(
                query_service,
                workspace_manager,
                database,
                event_bus,
            ),
        }
    }
}

impl ServerHandler for DefaultMcpService {
    async fn initialize(
        &self,
        request: InitializeRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        Ok(InitializeResult {
            protocol_version: request.protocol_version,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation::default(),
            instructions: None,
        })
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, ErrorData> {
        Ok(rmcp::model::ListToolsResult {
            tools: self.available_tools_service.get_available_tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::CallToolResult, ErrorData> {
        self.available_tools_service
            .call_tool(request.name.as_ref(), request.arguments.unwrap_or_default())
    }
}
