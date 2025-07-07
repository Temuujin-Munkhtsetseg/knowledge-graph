use crate::service::DefaultMcpService;
use database::querying::types::QueryingService;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use std::sync::Arc;
use workspace_manager::WorkspaceManager;

pub fn mcp_http_service(
    query_service: Arc<dyn QueryingService>,
    workspace_manager: Arc<WorkspaceManager>,
) -> StreamableHttpService<DefaultMcpService> {
    StreamableHttpService::new(
        move || {
            Ok(DefaultMcpService::new(
                Arc::clone(&query_service),
                Arc::clone(&workspace_manager),
            ))
        },
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    )
}
