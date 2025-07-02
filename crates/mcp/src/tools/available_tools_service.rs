use std::{borrow::Cow, collections::HashMap, sync::Arc};

use querying::{Query, QueryLibrary, QueryingService};
use rmcp::model::{CallToolResult, Content, JsonObject, Tool};
use serde_json::Map;

pub struct AvailableToolsService {
    tools: Vec<Tool>,
    tool_queries: HashMap<String, Query>,
    query_service: Arc<QueryingService>,
}

impl AvailableToolsService {
    pub fn new(query_service: Arc<QueryingService>) -> Self {
        let tools = QueryLibrary::all_queries()
            .into_iter()
            .map(|query| Tool {
                name: Cow::Borrowed(query.slug),
                description: Cow::Borrowed(query.description),
                input_schema: Arc::new(query_schema_to_input_schema(query.parameters.clone())),
            })
            .collect();

        let tool_queries = QueryLibrary::all_queries()
            .into_iter()
            .map(|query| (query.slug.to_string(), query))
            .collect();

        Self {
            tools,
            tool_queries,
            query_service,
        }
    }

    pub fn get_available_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    pub fn call_tool(
        &self,
        tool_name: &str,
        params: Option<JsonObject>,
    ) -> Result<CallToolResult, rmcp::Error> {
        if self.tool_queries.contains_key(tool_name) {
            return self.call_query_tool(self.tool_queries.get(tool_name).unwrap().clone(), params);
        }

        Err(rmcp::Error::new(
            rmcp::model::ErrorCode::RESOURCE_NOT_FOUND,
            format!("Tool {tool_name} not found"),
            None,
        ))
    }

    pub fn call_query_tool(
        &self,
        query: Query,
        params: Option<JsonObject>,
    ) -> Result<CallToolResult, rmcp::Error> {
        if params.is_none() {
            return Err(rmcp::Error::new(
                rmcp::model::ErrorCode::INVALID_PARAMS,
                "Params must be an object",
                None,
            ));
        }

        let mut param_object = params.unwrap();
        let project = param_object
            .remove("project")
            .and_then(|p| p.as_str().map(|s| s.to_string()))
            .ok_or_else(|| {
                rmcp::Error::new(
                    rmcp::model::ErrorCode::INVALID_PARAMS,
                    "Project is required",
                    None,
                )
            })?;

        self.query_service
            .execute_query(&project, query.query, param_object.clone())
            .and_then(|mut result| result.to_json())
            .map(|result| CallToolResult::success(vec![Content::json(result).unwrap()]))
            .map_err(|e| {
                rmcp::Error::new(
                    rmcp::model::ErrorCode::INTERNAL_ERROR,
                    format!("Error calling tool {}: {}", query.slug, e),
                    None,
                )
            })
    }
}

fn query_schema_to_input_schema(query_schema: serde_json::Value) -> Map<String, serde_json::Value> {
    let mut input_schema = Map::new();
    let mut required_keys = Vec::new();

    for (key, value) in query_schema.as_object().unwrap() {
        if value.get("required").is_some() {
            required_keys.push(key.clone());
        }

        input_schema.insert(key.clone(), value.clone());
    }

    input_schema.insert("project".to_string(), serde_json::json!({
        "type": "string",
        "required": true,
        "description": "The project path to query against. This is the path to the project root directory.",
    }));

    input_schema.insert("required".to_string(), serde_json::json!(required_keys));

    input_schema
}
