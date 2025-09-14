use crate::{configuration::McpConfiguration, service::DefaultMcpService};
use database::kuzu::database::KuzuDatabase;
use database::querying::types::QueryingService;
use event_bus::EventBus;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::Arc;
use workspace_manager::WorkspaceManager;

pub fn mcp_http_service(
    query_service: Arc<dyn QueryingService>,
    workspace_manager: Arc<WorkspaceManager>,
    database: Arc<KuzuDatabase>,
    event_bus: Arc<EventBus>,
    configuration: Arc<McpConfiguration>,
) -> StreamableHttpService<DefaultMcpService> {
    StreamableHttpService::new(
        move || {
            Ok(DefaultMcpService::new(
                Arc::clone(&query_service),
                Arc::clone(&workspace_manager),
                Arc::clone(&database),
                Arc::clone(&event_bus),
                Arc::clone(&configuration),
            ))
        },
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    )
}
