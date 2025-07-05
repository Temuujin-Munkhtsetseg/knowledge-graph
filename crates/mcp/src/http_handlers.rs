use crate::service::McpService;
use rmcp::model::{
    CallToolRequest, ErrorCode, ErrorData, InitializeRequest, JsonObject, JsonRpcMessage,
    NumberOrString, PaginatedRequestParam, object,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, error};

pub fn handle_mcp_request(service: &dyn McpService, request: Value) -> Value {
    debug!("[MCP] Received request: {}.", request);

    let method = request["method"].as_str().unwrap();
    let message: JsonRpcMessage = match method {
        "initialize" => {
            let id = extract_id(&request);
            let request = match parse_request::<InitializeRequest>(request) {
                Ok(request) => request,
                Err(error) => return error,
            };

            let result = service.initialize(request);

            JsonRpcMessage::response(to_json_object(result), id)
        }
        "notifications/initialized" => {
            service.on_initialized();

            return json!({});
        }
        "tools/list" => {
            let id = extract_id(&request);

            let request = match parse_request::<PaginatedRequestParam>(request) {
                Ok(request) => request,
                Err(error) => return error,
            };

            let result = service.list_tools(request);

            match result {
                Ok(tools) => JsonRpcMessage::response(to_json_object(tools), id),
                Err(error) => JsonRpcMessage::error(error, id),
            }
        }
        "tools/call" => {
            let id = extract_id(&request);

            let request = match parse_request::<CallToolRequest>(request) {
                Ok(request) => request,
                Err(error) => return error,
            };

            let result = service.call_tool(request.params);

            match result {
                Ok(result) => JsonRpcMessage::response(to_json_object(result), id),
                Err(error) => JsonRpcMessage::error(error, id),
            }
        }
        _ => JsonRpcMessage::error(
            ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Method not found: {method}"),
                None,
            ),
            extract_id(&request),
        ),
    };

    serde_json::to_value(message).unwrap_or_else(|e| {
        error!("[MCP] Failed to convert response message to JSON: {}.", e);
        json!({})
    })
}

pub fn handle_mcp_batch(requests: Vec<Value>, service: &dyn McpService) -> Vec<Value> {
    let mut responses = Vec::with_capacity(requests.len());

    for request in requests {
        let response = handle_mcp_request(service, request);
        responses.push(response);
    }

    responses
}

fn parse_request<T: for<'a> Deserialize<'a>>(request: Value) -> Result<T, Value> {
    let id = extract_id(&request);
    let json = serde_json::from_value::<T>(request);

    if json.is_err() {
        error!("[MCP] Failed to parse request: {}.", json.err().unwrap());

        let message: JsonRpcMessage = JsonRpcMessage::error(
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Invalid request parameters".to_string(),
                None,
            ),
            id,
        );

        return Err(serde_json::to_value(message).unwrap());
    }

    Ok(json.unwrap())
}

fn extract_id(request: &Value) -> NumberOrString {
    serde_json::from_value::<NumberOrString>(request["id"].clone())
        .unwrap_or(NumberOrString::Number(0))
}

fn to_json_object<T: Serialize>(result: T) -> JsonObject {
    object(serde_json::to_value(result).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::Error;
    use rmcp::model::{
        CallToolRequestParam, CallToolResult, Content, InitializeResult, ListToolsResult, Tool,
    };

    #[derive(Debug, Clone)]
    struct MockMcpService;

    impl McpService for MockMcpService {
        fn initialize(&self, _request: InitializeRequest) -> InitializeResult {
            InitializeResult::default()
        }

        fn on_initialized(&self) {}

        fn list_tools(&self, _request: PaginatedRequestParam) -> Result<ListToolsResult, Error> {
            // return a list of 4 tools
            Ok(ListToolsResult {
                tools: vec![
                    Tool::new(
                        "test".to_string(),
                        "test".to_string(),
                        serde_json::json!({}).as_object().unwrap().clone(),
                    ),
                    Tool::new(
                        "test2".to_string(),
                        "test2".to_string(),
                        serde_json::json!({}).as_object().unwrap().clone(),
                    ),
                    Tool::new(
                        "test3".to_string(),
                        "test3".to_string(),
                        serde_json::json!({}).as_object().unwrap().clone(),
                    ),
                    Tool::new(
                        "test4".to_string(),
                        "test4".to_string(),
                        serde_json::json!({}).as_object().unwrap().clone(),
                    ),
                ],
                next_cursor: None,
            })
        }

        fn call_tool(&self, _request: CallToolRequestParam) -> Result<CallToolResult, Error> {
            Ok(CallToolResult::success(vec![Content::text(
                "test".to_string(),
            )]))
        }
    }

    #[test]
    fn test_handle_initialize_request() {
        let service = MockMcpService {};
        let request = serde_json::json!({
            "id": 0,
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "capabilities": {},
                "clientInfo": {
                    "name": "gitlab-language-server",
                    "version": "1.0.0"
                },
                "protocolVersion": "2025-03-26"
            }
        });

        let response = handle_mcp_request(&service, request);

        assert_eq!(response["id"], 0);
    }

    #[test]
    fn test_on_initialized() {
        let service = MockMcpService {};
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        let response = handle_mcp_request(&service, request);

        assert_eq!(response, serde_json::json!({}));
    }

    #[test]
    fn test_list_tools() {
        let service = MockMcpService {};
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        });

        let response = handle_mcp_request(&service, request);

        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_list_tools_with_pagination() {
        let service = MockMcpService {};
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {
                "cursor": "1"
            }
        });

        let response = handle_mcp_request(&service, request);

        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_call_tool() {
        let service = MockMcpService {};
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "knowledge-graph-neighbours",
                "arguments": {
                    "fqn": "com.example.User",
                    "max_results": 10
                }
            }
        });

        let response = handle_mcp_request(&service, request);

        assert_eq!(response["id"], 1);
    }

    #[test]
    fn test_call_tool_with_invalid_arguments() {
        let service = MockMcpService {};
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "invalidRequestParam": {},
            }
        });

        let response = handle_mcp_request(&service, request);

        assert_eq!(response["id"], 1);
        assert_eq!(response["error"]["code"], -32602);
        assert_eq!(response["error"]["message"], "Invalid request parameters");
    }

    #[test]
    fn test_call_batch_of_tools() {
        let service = MockMcpService {};
        let request1 = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "knowledge-graph-neighbours",
                "arguments": {
                    "fqn": "com.example.User",
                    "max_results": 10
                }
            }
        });

        let request2 = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "knowledge-graph-neighbours",
                "arguments": {
                    "fqn": "com.example.User",
                    "max_results": 6
                }
            }
        });

        let response = handle_mcp_batch(vec![request1, request2], &service);

        assert_eq!(response[0]["id"], 1);
        assert_eq!(response[1]["id"], 2);
    }
}
