use crate::endpoints::shared::StatusResponse;
use database::querying::QueryResultRow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct DirectoryNodeProperties {
    pub path: String,
    pub absolute_path: String,
    pub repository_name: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct FileNodeProperties {
    pub path: String,
    pub absolute_path: String,
    pub repository_name: String,
    pub language: String,
    pub extension: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct DefinitionNodeProperties {
    pub path: String,
    pub fqn: String,
    pub definition_type: String,
    pub start_line: i32,
    pub primary_start_byte: i64,
    pub primary_end_byte: i64,
    pub total_locations: i32,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct ImportedSymbolNodeProperties {
    pub path: String,
    pub start_line: i32,
    pub primary_start_byte: i64,
    pub primary_end_byte: i64,
    pub import_type: String,
    pub import_path: String,
    pub import_alias: String,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
#[serde(tag = "node_type")]
pub enum TypedGraphNode {
    DirectoryNode {
        id: String,
        node_id: String,
        label: String,
        properties: DirectoryNodeProperties,
    },
    FileNode {
        id: String,
        node_id: String,
        label: String,
        properties: FileNodeProperties,
    },
    DefinitionNode {
        id: String,
        node_id: String,
        label: String,
        properties: DefinitionNodeProperties,
    },
    ImportedSymbolNode {
        id: String,
        node_id: String,
        label: String,
        properties: ImportedSymbolNodeProperties,
    },
}

#[derive(Serialize, Deserialize, TS, Default, Debug)]
#[ts(export, export_to = "../../../packages/gkg/src/api.ts")]
pub struct GraphRelationship {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relationship_type: String,
    #[ts(type = "Record<string, any>")]
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct NodeData {
    pub id: String,
    pub node_id: String,
    pub node_type: String,
    pub name: String,
    pub path: String,
    pub absolute_path: String,
    pub repository_name: String,
    pub fqn: String,
    pub definition_type: String,
    pub language: String,
    pub extension: String,
    pub start_line: i64,
    pub primary_start_byte: i64,
    pub primary_end_byte: i64,
    pub total_locations: i64,
    pub import_type: String,
    pub import_path: String,
    pub import_alias: String,
}

pub fn extract_node_data(
    row: &dyn QueryResultRow,
    start_index: usize,
) -> Result<NodeData, Box<dyn std::error::Error>> {
    let node_id = row.get_string_value(start_index)?;
    let node_type = row.get_string_value(start_index + 1)?;
    Ok(NodeData {
        id: format!("{node_type}_{node_id}"),
        node_id,
        node_type,
        name: row.get_string_value(start_index + 2)?,
        path: row.get_string_value(start_index + 3)?,
        absolute_path: row.get_string_value(start_index + 4)?,
        repository_name: row.get_string_value(start_index + 5)?,
        fqn: row.get_string_value(start_index + 6)?,
        definition_type: row.get_string_value(start_index + 7)?,
        language: row.get_string_value(start_index + 8)?,
        extension: row.get_string_value(start_index + 9)?,
        start_line: row.get_int_value(start_index + 10)?,
        primary_start_byte: row.get_int_value(start_index + 11)?,
        primary_end_byte: row.get_int_value(start_index + 12)?,
        total_locations: row.get_int_value(start_index + 13)?,
        import_type: row.get_string_value(start_index + 14)?,
        import_path: row.get_string_value(start_index + 15)?,
        import_alias: row.get_string_value(start_index + 16)?,
    })
}

pub fn create_typed_node(data: NodeData) -> Result<TypedGraphNode, Box<dyn std::error::Error>> {
    let node = match data.node_type.as_str() {
        "DirectoryNode" => TypedGraphNode::DirectoryNode {
            id: data.id,
            node_id: data.node_id,
            label: data.name,
            properties: DirectoryNodeProperties {
                path: data.path,
                absolute_path: data.absolute_path,
                repository_name: data.repository_name,
            },
        },
        "FileNode" => TypedGraphNode::FileNode {
            id: data.id,
            node_id: data.node_id,
            label: data.name,
            properties: FileNodeProperties {
                path: data.path,
                absolute_path: data.absolute_path,
                repository_name: data.repository_name,
                language: data.language,
                extension: data.extension,
            },
        },
        "DefinitionNode" => TypedGraphNode::DefinitionNode {
            id: data.id,
            node_id: data.node_id,
            label: data.name,
            properties: DefinitionNodeProperties {
                path: data.path,
                fqn: data.fqn,
                definition_type: data.definition_type,
                start_line: data.start_line as i32,
                primary_start_byte: data.primary_start_byte,
                primary_end_byte: data.primary_end_byte,
                total_locations: data.total_locations as i32,
            },
        },
        "ImportedSymbolNode" => TypedGraphNode::ImportedSymbolNode {
            id: data.id,
            node_id: data.node_id,
            label: data.name,
            properties: ImportedSymbolNodeProperties {
                path: data.path,
                start_line: data.start_line as i32,
                primary_start_byte: data.primary_start_byte,
                primary_end_byte: data.primary_end_byte,
                import_type: data.import_type,
                import_path: data.import_path,
                import_alias: data.import_alias,
            },
        },
        _ => TypedGraphNode::DirectoryNode {
            id: data.id,
            node_id: data.node_id,
            label: data.name,
            properties: DirectoryNodeProperties {
                path: data.path,
                absolute_path: data.absolute_path,
                repository_name: data.repository_name,
            },
        },
    };

    Ok(node)
}

#[macro_export]
macro_rules! decode_url_param {
    ($param:expr, $param_name:literal, $error_handler:expr) => {
        match urlencoding::decode($param) {
            Ok(decoded) => decoded.into_owned(),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json($error_handler(
                        concat!("invalid_", $param_name, "_encoding").to_string(),
                    )),
                )
                    .into_response();
            }
        }
    };
}

pub fn create_error_response(status: String) -> StatusResponse {
    StatusResponse { status }
}
