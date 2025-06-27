// TODO: This is placeholder code for future implementation
// The tool definitions will be replaced with actual knowledge graph tool schemas
use crate::types::*;

pub fn get_available_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "knowledge-graph-neighbours".to_string(),
            description: "Get neighbouring entities for a fully qualified name (FQN) in the knowledge graph. Returns related classes, functions, and dependencies.".to_string(),
            input_schema: serde_json::json!({
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
            }),
            annotations: ToolAnnotations {
                title: "Knowledge Graph Neighbours".to_string(),
                read_only_hint: true,
                open_world_hint: false,
            },
        },
        ToolDefinition {
            name: "knowledge-graph-search".to_string(),
            description: "Search the knowledge graph for entities matching the query. Supports semantic search across classes, functions, and documentation.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { 
                        "type": "string",
                        "description": "The search query to find matching entities"
                    },
                    "type": {
                        "type": "string",
                        "description": "Filter by entity type (class, function, variable, etc.)",
                        "enum": ["class", "function", "variable", "all"]
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return",
                        "default": 20
                    }
                },
                "required": ["query"]
            }),
            annotations: ToolAnnotations {
                title: "Knowledge Graph Search".to_string(),
                read_only_hint: true,
                open_world_hint: false,
            },
        },
        ToolDefinition {
            name: "knowledge-graph-entity-details".to_string(),
            description: "Get detailed information about a specific entity in the knowledge graph, including its properties, methods, and relationships.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "fqn": { 
                        "type": "string",
                        "description": "The fully qualified name of the entity"
                    }
                },
                "required": ["fqn"]
            }),
            annotations: ToolAnnotations {
                title: "Entity Details".to_string(),
                read_only_hint: true,
                open_world_hint: false,
            },
        },
        ToolDefinition {
            name: "knowledge-graph-dependency-path".to_string(),
            description: "Find the dependency path between two entities in the knowledge graph, showing how they are connected through intermediate entities.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source": { 
                        "type": "string",
                        "description": "The source entity's fully qualified name"
                    },
                    "target": { 
                        "type": "string",
                        "description": "The target entity's fully qualified name"
                    },
                    "max_path_length": {
                        "type": "integer",
                        "description": "Maximum path length to search",
                        "default": 5
                    }
                },
                "required": ["source", "target"]
            }),
            annotations: ToolAnnotations {
                title: "Dependency Path".to_string(),
                read_only_hint: true,
                open_world_hint: false,
            },
        },
    ]
}
