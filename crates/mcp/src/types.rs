/// A response containing a list of available tools.
#[derive(serde::Serialize)]
pub struct ListToolsResponse {
    /// A list of tool definitions.
    pub tools: Vec<ToolDefinition>,
}

/// Defines a tool that can be called by a client.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    /// The unique name of the tool.
    pub name: String,
    /// A description of what the tool does.
    pub description: String,
    /// A JSON schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
    /// Annotations providing hints about the tool's behavior.
    pub annotations: ToolAnnotations,
}

/// Annotations that provide metadata about a tool's behavior.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// A human-readable title for the tool.
    pub title: String,
    /// A hint that the tool does not modify its environment.
    pub read_only_hint: bool,
    /// A hint that the tool interacts with external entities.
    pub open_world_hint: bool,
}

/// A request to call a specific tool.
#[derive(serde::Deserialize)]
pub struct CallToolRequest {
    /// The name of the tool to call.
    pub name: String,
    /// The arguments for the tool, as a JSON value.
    pub arguments: serde_json::Value,
}

/// A response from a tool call.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResponse {
    /// The content of the response.
    pub content: Vec<TextContent>,
    /// Whether the tool call resulted in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Text content for a tool call response.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TextContent {
    /// The type of the content, always "text".
    pub r#type: String,
    /// The text content.
    pub text: String,
}

// MCP-specific response structures
#[derive(serde::Serialize)]
pub struct McpResponse<T> {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

#[derive(serde::Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
pub struct McpBatchResponse {
    pub responses: Vec<McpResponse<serde_json::Value>>,
}
