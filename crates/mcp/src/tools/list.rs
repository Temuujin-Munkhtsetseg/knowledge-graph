// TODO: This is placeholder code for future implementation
// The tool definitions will be replaced with actual knowledge graph tool schemas
use rmcp::model::{JsonObject, Tool};
use serde_json::Value;
use serde_json::json;
use std::{borrow::Cow, sync::Arc};

pub fn get_available_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: Cow::Borrowed("knowledge-graph-neighbours"),
            description: Cow::Borrowed(
                "Get neighbouring entities for a fully qualified name (FQN) in the knowledge graph. Returns related classes, functions, and dependencies.",
            ),
            input_schema: Arc::new(input_schema(json!({
                "type": "object",
                "properties": {
                    "fqn": {
                        "type": "string",
                        "description": "The fully qualified name to find neighbours for (e.g., 'com.example.User')"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of neighbours to return",
                        "default": 10
                    }
                },
                "required": ["fqn"]
            }))),
        },
        Tool {
            name: Cow::Borrowed("knowledge-graph-search"),
            description: Cow::Borrowed(
                "Search the knowledge graph for entities matching a given query.",
            ),
            input_schema: Arc::new(input_schema(json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The query to search for (e.g., 'User')"
                    }
                },
                "required": ["query"]
            }))),
        },
        Tool {
            name: Cow::Borrowed("knowledge-graph-entity-details"),
            description: Cow::Borrowed(
                "Get detailed information about a specific entity in the knowledge graph.",
            ),
            input_schema: Arc::new(input_schema(json!({
                "type": "object",
                "properties": {
                    "fqn": {
                        "type": "string",
                        "description": "The fully qualified name of the entity to get details for (e.g., 'com.example.User')"
                    }
                },
                "required": ["fqn"]
            }))),
        },
        Tool {
            name: Cow::Borrowed("knowledge-graph-dependency-path"),
            description: Cow::Borrowed(
                "Get the dependency path between two entities in the knowledge graph.",
            ),
            input_schema: Arc::new(input_schema(json!({
                "type": "object",
                "properties": {
                    "fqn1": {
                        "type": "string",
                        "description": "The fully qualified name of the first entity (e.g., 'com.example.User')"
                    },
                    "fqn2": {
                        "type": "string",
                        "description": "The fully qualified name of the second entity (e.g., 'com.example.User')"
                    }
                },
                "required": ["fqn1", "fqn2"]
            }))),
        },
    ]
}

fn input_schema(value: Value) -> JsonObject {
    value.as_object().unwrap().clone()
}
