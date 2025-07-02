use anyhow::{Error, Result};
use database::{DatabaseConnection, DatabaseResult};
use serde_json::Map;
use std::sync::Arc;
use workspace_manager::WorkspaceManager;

pub struct QueryingService {
    connection: Box<dyn DatabaseConnection>,
    workspace_manager: Arc<WorkspaceManager>,
}

// This service should only be used for uncontrolled query execution (e.g., MCP, Playground, API endpoints).
// For controlled query execution with strict typing for arguments and return types, a proper service should be created instead.
impl QueryingService {
    pub fn new(
        connection: Box<dyn DatabaseConnection>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            connection,
            workspace_manager,
        }
    }

    pub fn execute_query(
        &self,
        project_path: &str,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<Box<dyn DatabaseResult>, Error> {
        let project = self.workspace_manager.get_project_for_path(project_path);

        if project.is_none() {
            return Err(Error::msg(format!(
                "Project not found for path: {project_path}"
            )));
        }

        self.connection.query(
            project.unwrap().database_path.to_str().unwrap(),
            query,
            params,
        )
    }
}
