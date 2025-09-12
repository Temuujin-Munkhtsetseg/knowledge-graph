use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::ANALYZE_CODE_FILES_TOOL_NAME;
use crate::tools::INDEX_PROJECT_TOOL_NAME;
use crate::tools::SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME;
use crate::tools::SearchCodebaseDefinitionsTool;
use crate::tools::analyze_code_files::AnalyzeCodeFilesTool;
use crate::tools::get_definition::GetDefinitionTool;
use crate::tools::get_definition::constants::GET_DEFINITION_TOOL_NAME;
use crate::tools::get_references::GET_REFERENCES_TOOL_NAME;
use crate::tools::get_references::tool::GetReferencesTool;
use crate::tools::index_project::IndexProjectTool;
use crate::tools::types::KnowledgeGraphTool;
use crate::tools::workspace_tools::get_list_projects_tool;
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
    ) -> Self {
        let mut tools: HashMap<String, Box<dyn KnowledgeGraphTool>> = HashMap::new();

        let list_projects_tool = get_list_projects_tool(workspace_manager.clone());

        tools.insert(
            list_projects_tool.name().to_string(),
            Box::new(list_projects_tool),
        );

        tools.insert(
            SEARCH_CODEBASE_DEFINITIONS_TOOL_NAME.to_string(),
            Box::new(SearchCodebaseDefinitionsTool::new(
                query_service.clone(),
                workspace_manager.clone(),
            )),
        );

        tools.insert(
            ANALYZE_CODE_FILES_TOOL_NAME.to_string(),
            Box::new(AnalyzeCodeFilesTool::new(
                query_service.clone(),
                workspace_manager.clone(),
            )),
        );

        tools.insert(
            INDEX_PROJECT_TOOL_NAME.to_string(),
            Box::new(IndexProjectTool::new(
                database.clone(),
                workspace_manager.clone(),
                event_bus.clone(),
            )),
        );

        tools.insert(
            GET_REFERENCES_TOOL_NAME.to_string(),
            Box::new(GetReferencesTool::new(
                query_service.clone(),
                workspace_manager.clone(),
            )),
        );

        tools.insert(
            GET_DEFINITION_TOOL_NAME.to_string(),
            Box::new(GetDefinitionTool::new(
                database.clone(),
                workspace_manager.clone(),
            )),
        );

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
