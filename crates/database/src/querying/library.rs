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

    pub fn all_queries() -> Vec<Query> {
        vec![
            Self::get_definition_relations_query(),
            Self::get_file_definitions_query(),
            Self::get_list_matches_query(),
        ]
    }
}
