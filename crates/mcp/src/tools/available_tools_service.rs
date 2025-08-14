use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::ANALYZE_CODE_FILES_TOOL_NAME;
use crate::tools::analyze_code_files::AnalyzeCodeFilesTool;
use crate::tools::search_codebase::SearchCodebaseTool;
use crate::tools::types::KnowledgeGraphTool;
use crate::tools::workspace_tools::get_list_projects_tool;
use database::querying::QueryingService;
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
    ) -> Self {
        let mut tools: HashMap<String, Box<dyn KnowledgeGraphTool>> = HashMap::new();

        let list_projects_tool = get_list_projects_tool(workspace_manager.clone());

        tools.insert(
            list_projects_tool.name().to_string(),
            Box::new(list_projects_tool),
        );

        tools.insert(
            "search_codebase".to_string(),
            Box::new(SearchCodebaseTool::new(
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

        Self { tools }
    }

    pub fn get_available_tools(&self) -> Vec<Tool> {
        self.tools.values().map(|tool| tool.to_mcp_tool()).collect()
    }

    pub fn call_tool(
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
