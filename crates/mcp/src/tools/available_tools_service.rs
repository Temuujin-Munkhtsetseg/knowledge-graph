use std::collections::HashMap;
use std::sync::Arc;

use crate::configuration::McpConfiguration;
use crate::tools::INDEX_PROJECT_TOOL_NAME;
use crate::tools::SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME;
use crate::tools::SearchCodebaseDefinitionsTool;
use crate::tools::get_definition::GetDefinitionTool;
use crate::tools::get_definition::constants::GET_DEFINITION_TOOL_NAME;
use crate::tools::get_references::GET_REFERENCES_TOOL_NAME;
use crate::tools::get_references::tool::GetReferencesTool;
use crate::tools::index_project::IndexProjectTool;
use crate::tools::list_projects::{LIST_PROJECTS_TOOL_NAME, ListProjectsTool};
use crate::tools::read_definitions::READ_DEFINITIONS_TOOL_NAME;
use crate::tools::read_definitions::tool::ReadDefinitionsTool;
use crate::tools::repo_map::{REPO_MAP_TOOL_NAME, RepoMapTool};
use crate::tools::types::KnowledgeGraphTool;
use database::kuzu::database::KuzuDatabase;
use database::querying::QueryingService;
use event_bus::EventBus;
use rmcp::model::CallToolResult;
use rmcp::model::JsonObject;
use rmcp::model::Tool;
use workspace_manager::WorkspaceManager;

pub struct AvailableToolsService {
    tools: HashMap<String, Box<dyn KnowledgeGraphTool>>,
}

impl AvailableToolsService {
    pub fn new(
        query_service: Arc<dyn QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
        database: Arc<KuzuDatabase>,
        event_bus: Arc<EventBus>,
        configuration: Arc<McpConfiguration>,
    ) -> Self {
        let mut tools: HashMap<String, Box<dyn KnowledgeGraphTool>> = HashMap::new();

        if configuration.is_tool_enabled(LIST_PROJECTS_TOOL_NAME) {
            tools.insert(
                LIST_PROJECTS_TOOL_NAME.to_string(),
                Box::new(ListProjectsTool::new(workspace_manager.clone())),
            );
        }

        if configuration.is_tool_enabled(SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME) {
            tools.insert(
                SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME.to_string(),
                Box::new(SearchCodebaseDefinitionsTool::new(
                    query_service.clone(),
                    workspace_manager.clone(),
                )),
            );
        }

        if configuration.is_tool_enabled(INDEX_PROJECT_TOOL_NAME) {
            tools.insert(
                INDEX_PROJECT_TOOL_NAME.to_string(),
                Box::new(IndexProjectTool::new(
                    database.clone(),
                    workspace_manager.clone(),
                    event_bus.clone(),
                )),
            );
        }

        if configuration.is_tool_enabled(GET_REFERENCES_TOOL_NAME) {
            tools.insert(
                GET_REFERENCES_TOOL_NAME.to_string(),
                Box::new(GetReferencesTool::new(
                    query_service.clone(),
                    workspace_manager.clone(),
                )),
            );
        }

        if configuration.is_tool_enabled(GET_DEFINITION_TOOL_NAME) {
            tools.insert(
                GET_DEFINITION_TOOL_NAME.to_string(),
                Box::new(GetDefinitionTool::new(
                    database.clone(),
                    workspace_manager.clone(),
                )),
            );
        }

        if configuration.is_tool_enabled(READ_DEFINITIONS_TOOL_NAME) {
            tools.insert(
                READ_DEFINITIONS_TOOL_NAME.to_string(),
                Box::new(ReadDefinitionsTool::new(
                    query_service.clone(),
                    workspace_manager.clone(),
                )),
            );
        }

        if configuration.is_tool_enabled(REPO_MAP_TOOL_NAME) {
            tools.insert(
                REPO_MAP_TOOL_NAME.to_string(),
                Box::new(RepoMapTool::new(
                    query_service.clone(),
                    workspace_manager.clone(),
                )),
            );
        }

        Self { tools }
    }

    pub fn get_available_tools(&self) -> Vec<Tool> {
        self.tools.values().map(|tool| tool.to_mcp_tool()).collect()
    }

    pub async fn call_tool(
        &self,
        tool_name: &str,
        params: JsonObject,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.tools
            .get(tool_name)
            .ok_or(rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                format!("Tool {tool_name} not found."),
                None,
            ))?
            .call(params)
    }
}
