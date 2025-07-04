use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::query_tools::QueryKnowledgeGraphTool;
use crate::tools::types::KnowledgeGraphTool;
use database::querying::{Query, QueryLibrary, QueryingService};
use rmcp::model::CallToolResult;
use rmcp::model::JsonObject;
use rmcp::model::Tool;

pub struct AvailableToolsService {
    tools: HashMap<String, Box<dyn KnowledgeGraphTool>>,
}

impl AvailableToolsService {
    pub fn new(query_service: Arc<dyn QueryingService>) -> Self {
        let mut tools: HashMap<String, Box<dyn KnowledgeGraphTool>> = HashMap::new();

        add_query_tool(
            &mut tools,
            QueryLibrary::get_definition_relations_query(),
            query_service.clone(),
        );

        add_query_tool(
            &mut tools,
            QueryLibrary::get_file_definitions_query(),
            query_service.clone(),
        );

        add_query_tool(
            &mut tools,
            QueryLibrary::get_list_matches_query(),
            query_service.clone(),
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
    ) -> Result<CallToolResult, rmcp::Error> {
        self.tools
            .get(tool_name)
            .ok_or(rmcp::Error::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                format!("Tool {tool_name} not found."),
                None,
            ))?
            .call(params)
    }
}

fn add_query_tool(
    tools: &mut HashMap<String, Box<dyn KnowledgeGraphTool>>,
    query: Query,
    query_service: Arc<dyn QueryingService>,
) {
    tools.insert(
        query.name.to_string(),
        Box::new(QueryKnowledgeGraphTool::new(query_service.clone(), query)),
    );
}
