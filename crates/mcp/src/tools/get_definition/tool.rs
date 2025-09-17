use std::borrow::Cow;
use std::sync::Arc;

use database::kuzu::database::KuzuDatabase;
use rmcp::model::{CallToolResult, Content, JsonObject, Tool, object};
use serde_json::json;
use workspace_manager::WorkspaceManager;

use super::constants::{GET_DEFINITION_TOOL_DESCRIPTION, GET_DEFINITION_TOOL_NAME};
use super::input::GetDefinitionInput;
use super::service::GetDefinitionService;
use crate::tools::types::KnowledgeGraphTool;
use crate::tools::xml::ToXml;

pub struct GetDefinitionTool {
    service: GetDefinitionService,
}

impl GetDefinitionTool {
    pub fn new(database: Arc<KuzuDatabase>, workspace_manager: Arc<WorkspaceManager>) -> Self {
        Self {
            service: GetDefinitionService::new(database, workspace_manager),
        }
    }
}

#[async_trait::async_trait]
impl KnowledgeGraphTool for GetDefinitionTool {
    fn name(&self) -> &str {
        GET_DEFINITION_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let input_schema = json!({
            "type": "object",
            "properties": {
                "absolute_file_path": {
                    "type": "string",
                    "description": "Absolute file path to the file that contains the symbol usage. Example: /abs/path/to/src/main/java/com/example/User.java"
                },
                "line": {
                    "type": "string",
                    "description": "Exact line of code copied from the file (whitespace must match). Example: var name = user.getFirstName() + user.getLastName();"
                },
                "symbol_name": {
                    "type": "string",
                    "description": "Callable symbol to resolve (method/function name). Example: getFirstName"
                }
            },
            "required": ["absolute_file_path", "line", "symbol_name"]
        });

        Tool {
            name: Cow::Borrowed(GET_DEFINITION_TOOL_NAME),
            description: Some(Cow::Borrowed(GET_DEFINITION_TOOL_DESCRIPTION)),
            input_schema: Arc::new(object(input_schema)),
            output_schema: None,
            annotations: None,
        }
    }

    async fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let input = GetDefinitionInput::try_from(params)?;

        let result = self.service.get_definition(input).await?;

        let xml_output = result.to_xml_without_cdata().map_err(|e| {
            rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INTERNAL_ERROR,
                format!("Failed to convert output to XML: {}", e),
                None,
            )
        })?;

        Ok(CallToolResult::success(vec![Content::text(xml_output)]))
    }
}
