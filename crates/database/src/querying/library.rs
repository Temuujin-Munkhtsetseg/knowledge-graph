use std::collections::HashMap;

use crate::querying::mappers::{
    INT_MAPPER, QueryResultMapper, RELATIONSHIP_TYPE_MAPPER, STRING_MAPPER,
};

pub struct QueryLibrary;

#[derive(Debug, Clone)]
pub struct Query {
    pub name: &'static str,
    pub description: &'static str,
    pub query: &'static str,
    pub parameters: HashMap<&'static str, QueryParameter>,
    pub result: HashMap<&'static str, QueryResultMapper>,
}

#[derive(Debug, Clone)]
pub struct QueryParameter {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub definition: QueryParameterDefinition,
}

#[derive(Debug, Clone)]
pub enum QueryParameterDefinition {
    String(Option<String>),
    Int(Option<i64>),
    Float(Option<f64>),
    Boolean(Option<bool>),
}

impl QueryLibrary {
    pub fn get_definition_relations_query() -> Query {
        Query {
            name: "list_relations",
            description: "Get all related definitions for any given definition.",
            query: r#"
                MATCH (n:DefinitionNode)-[r]-(related:DefinitionNode)
                WHERE n.fqn = $fqn
                RETURN
                    related.fqn as fqn,
                    r.type as relationship_type,
                    related.name as name,
                    related.definition_type as definition_type,
                    related.primary_file_path as file_path,
                    related.primary_line_number as line_number
                LIMIT $limit
            "#,
            parameters: HashMap::from([
                (
                    "fqn",
                    QueryParameter {
                        name: "fqn",
                        description: "The FQN of the definition to get relationships for.",
                        required: true,
                        definition: QueryParameterDefinition::String(None),
                    },
                ),
                (
                    "limit",
                    QueryParameter {
                        name: "limit",
                        description: "The maximum number of relationships to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(100)),
                    },
                ),
            ]),
            result: HashMap::from([
                ("fqn", STRING_MAPPER),
                ("relationship_type", RELATIONSHIP_TYPE_MAPPER),
                ("name", STRING_MAPPER),
                ("definition_type", STRING_MAPPER),
                ("file_path", STRING_MAPPER),
                ("line_number", INT_MAPPER),
            ]),
        }
    }

    pub fn get_file_definitions_query() -> Query {
        Query {
            name: "list_file_definitions",
            description: "List all definitions inside a specific file.",
            query: r#"
                MATCH (file:FileNode)-[r:FILE_RELATIONSHIPS]->(definition:DefinitionNode)
                WHERE file.path = $file_path OR file.absolute_path = $file_path
                RETURN
                    definition.fqn as fqn,
                    definition.name as name,
                    definition.definition_type as definition_type,
                    definition.primary_line_number as line_number,
                    definition.primary_file_path as file_path
                ORDER BY definition.primary_line_number
                LIMIT $limit
            "#,
            parameters: HashMap::from([
                (
                    "file_path",
                    QueryParameter {
                        name: "file_path",
                        description: "The path of the file to get definitions for.",
                        required: true,
                        definition: QueryParameterDefinition::String(None),
                    },
                ),
                (
                    "limit",
                    QueryParameter {
                        name: "limit",
                        description: "The maximum number of definitions to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(100)),
                    },
                ),
            ]),
            result: HashMap::from([
                ("fqn", STRING_MAPPER),
                ("name", STRING_MAPPER),
                ("definition_type", STRING_MAPPER),
                ("line_number", INT_MAPPER),
                ("file_path", STRING_MAPPER),
            ]),
        }
    }

    pub fn get_list_matches_query() -> Query {
        Query {
            name: "list_matches",
            description: "Get all definitions with FQNs that contain the provided string (case insensitive).",
            query: r#"
                MATCH (n:DefinitionNode)
                WHERE toLower(n.fqn) CONTAINS toLower($search_string)
                RETURN
                    n.fqn as fqn,
                    n.name as name,
                    n.definition_type as definition_type,
                    n.primary_file_path as file_path,
                    n.primary_line_number as line_number
                ORDER BY n.fqn
                LIMIT $limit
            "#,
            parameters: HashMap::from([
                (
                    "search_string",
                    QueryParameter {
                        name: "search_string",
                        description: "The string to search for within FQNs (case insensitive).",
                        required: true,
                        definition: QueryParameterDefinition::String(None),
                    },
                ),
                (
                    "limit",
                    QueryParameter {
                        name: "limit",
                        description: "The maximum number of matches to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(100)),
                    },
                ),
            ]),
            result: HashMap::from([
                ("fqn", STRING_MAPPER),
                ("name", STRING_MAPPER),
                ("definition_type", STRING_MAPPER),
                ("line_number", INT_MAPPER),
                ("file_path", STRING_MAPPER),
            ]),
        }
    }

    pub fn get_initial_project_graph_query() -> Query {
        Query {
            name: "initial_project_graph",
            description: "Get the initial graph of a project, including nodes and relationships. Used in the KG panel explorer.",
            query: r#"
                MATCH (n:DirectoryNode)-[r:DIRECTORY_RELATIONSHIPS]-(m:DirectoryNode)
                RETURN 
                    n.id as source_id, 
                    'DirectoryNode' as source_type,
                    n.name as source_name,
                    n.path as source_path,
                    n.absolute_path as source_absolute_path,
                    n.repository_name as source_repository_name,
                    '' as source_fqn,
                    '' as source_definition_type,
                    '' as source_language,
                    '' as source_extension,
                    CAST(0 AS INT64) as source_primary_line_number,
                    CAST(0 AS INT64) as source_primary_start_byte,
                    CAST(0 AS INT64) as source_primary_end_byte,
                    CAST(0 AS INT64) as source_total_locations,
                    m.id as target_id,
                    'DirectoryNode' as target_type,
                    m.name as target_name,
                    m.path as target_path,
                    m.absolute_path as target_absolute_path,
                    m.repository_name as target_repository_name,
                    '' as target_fqn,
                    '' as target_definition_type,
                    '' as target_language,
                    '' as target_extension,
                    CAST(0 AS INT64) as target_primary_line_number,
                    CAST(0 AS INT64) as target_primary_start_byte,
                    CAST(0 AS INT64) as target_primary_end_byte,
                    CAST(0 AS INT64) as target_total_locations,
                    'DIRECTORY_RELATIONSHIPS' as relationship_type,
                    id(r) as relationship_id,
                    1 as order_priority
                LIMIT $directory_limit
                UNION
                MATCH (f:FileNode)-[r:FILE_RELATIONSHIPS]-(d:DefinitionNode)
                RETURN 
                    f.id as source_id,
                    'FileNode' as source_type,
                    f.name as source_name,
                    f.path as source_path,
                    f.absolute_path as source_absolute_path,
                    f.repository_name as source_repository_name,
                    '' as source_fqn,
                    '' as source_definition_type,
                    f.language as source_language,
                    f.extension as source_extension,
                    CAST(0 AS INT64) as source_primary_line_number,
                    CAST(0 AS INT64) as source_primary_start_byte,
                    CAST(0 AS INT64) as source_primary_end_byte,
                    CAST(0 AS INT64) as source_total_locations,
                    d.id as target_id,
                    'DefinitionNode' as target_type,
                    d.name as target_name,
                    d.primary_file_path as target_path,
                    '' as target_absolute_path,
                    '' as target_repository_name,
                    d.fqn as target_fqn,
                    d.definition_type as target_definition_type,
                    '' as target_language,
                    '' as target_extension,
                    CAST(d.primary_line_number AS INT64) as target_primary_line_number,
                    d.primary_start_byte as target_primary_start_byte,
                    d.primary_end_byte as target_primary_end_byte,
                    CAST(d.total_locations AS INT64) as target_total_locations,
                    'FILE_RELATIONSHIPS' as relationship_type,
                    id(r) as relationship_id,
                    2 as order_priority
                LIMIT $file_limit
                UNION
                MATCH (d1:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]-(d2:DefinitionNode)
                RETURN 
                    d1.id as source_id,
                    'DefinitionNode' as source_type,
                    d1.name as source_name,
                    d1.primary_file_path as source_path,
                    '' as source_absolute_path,
                    '' as source_repository_name,
                    d1.fqn as source_fqn,
                    d1.definition_type as source_definition_type,
                    '' as source_language,
                    '' as source_extension,
                    CAST(d1.primary_line_number AS INT64) as source_primary_line_number,
                    d1.primary_start_byte as source_primary_start_byte,
                    d1.primary_end_byte as source_primary_end_byte,
                    CAST(d1.total_locations AS INT64) as source_total_locations,
                    d2.id as target_id,
                    'DefinitionNode' as target_type,
                    d2.name as target_name,
                    d2.primary_file_path as target_path,
                    '' as target_absolute_path,
                    '' as target_repository_name,
                    d2.fqn as target_fqn,
                    d2.definition_type as target_definition_type,
                    '' as target_language,
                    '' as target_extension,
                    CAST(d2.primary_line_number AS INT64) as target_primary_line_number,
                    d2.primary_start_byte as target_primary_start_byte,
                    d2.primary_end_byte as target_primary_end_byte,
                    CAST(d2.total_locations AS INT64) as target_total_locations,
                    'DEFINITION_RELATIONSHIPS' as relationship_type,
                    id(r) as relationship_id,
                    3 as order_priority
                LIMIT $definition_limit
            "#,
            parameters: HashMap::from([
                (
                    "directory_limit",
                    QueryParameter {
                        name: "directory_limit",
                        description: "The maximum number of directory relationships to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(50)),
                    },
                ),
                (
                    "file_limit",
                    QueryParameter {
                        name: "file_limit",
                        description: "The maximum number of file relationships to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(100)),
                    },
                ),
                (
                    "definition_limit",
                    QueryParameter {
                        name: "definition_limit",
                        description: "The maximum number of definition relationships to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(200)),
                    },
                ),
            ]),
            result: HashMap::from([
                ("source_id", STRING_MAPPER),
                ("source_type", STRING_MAPPER),
                ("source_name", STRING_MAPPER),
                ("source_path", STRING_MAPPER),
                ("source_absolute_path", STRING_MAPPER),
                ("source_repository_name", STRING_MAPPER),
                ("source_fqn", STRING_MAPPER),
                ("source_definition_type", STRING_MAPPER),
                ("source_language", STRING_MAPPER),
                ("source_extension", STRING_MAPPER),
                ("source_primary_line_number", INT_MAPPER),
                ("source_primary_start_byte", INT_MAPPER),
                ("source_primary_end_byte", INT_MAPPER),
                ("source_total_locations", INT_MAPPER),
                ("target_id", STRING_MAPPER),
                ("target_type", STRING_MAPPER),
                ("target_name", STRING_MAPPER),
                ("target_path", STRING_MAPPER),
                ("target_absolute_path", STRING_MAPPER),
                ("target_repository_name", STRING_MAPPER),
                ("target_fqn", STRING_MAPPER),
                ("target_definition_type", STRING_MAPPER),
                ("target_language", STRING_MAPPER),
                ("target_extension", STRING_MAPPER),
                ("target_primary_line_number", INT_MAPPER),
                ("target_primary_start_byte", INT_MAPPER),
                ("target_primary_end_byte", INT_MAPPER),
                ("target_total_locations", INT_MAPPER),
                ("relationship_type", STRING_MAPPER),
                ("relationship_id", STRING_MAPPER),
                ("order_priority", INT_MAPPER),
            ]),
        }
    }

    pub fn all_queries() -> Vec<Query> {
        vec![
            Self::get_definition_relations_query(),
            Self::get_file_definitions_query(),
            Self::get_list_matches_query(),
            Self::get_initial_project_graph_query(),
        ]
    }
}
