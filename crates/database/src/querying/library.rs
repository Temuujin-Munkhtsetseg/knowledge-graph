use std::collections::HashMap;

use crate::querying::mappers::{
    INT_MAPPER, QueryResultMapper, RELATIONSHIP_TYPE_MAPPER, STRING_MAPPER,
};

pub struct QueryLibrary;

#[derive(Debug, Clone)]
pub struct Query {
    pub query: String,
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

// TODO: Handle new ID for definitions (file_path + fqn)
// TODO: Add queries for imported symbols

impl QueryLibrary {
    pub fn get_definition_relations_query() -> Query {
        Query {
            query: r#"
                MATCH (n:DefinitionNode)-[r]-(related:DefinitionNode)
                WHERE n.fqn = $fqn
                RETURN
                    related.fqn as fqn,
                    r.type as relationship_type,
                    related.name as name,
                    related.definition_type as definition_type,
                    related.primary_file_path as file_path,
                    related.start_line as line_number
                LIMIT $limit
            "#
            .to_string(),
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
            query: r#"
                MATCH (file:FileNode)-[r:FILE_RELATIONSHIPS]->(definition:DefinitionNode)
                WHERE file.path = $file_path OR file.absolute_path = $file_path
                RETURN
                    definition.fqn as fqn,
                    definition.name as name,
                    definition.definition_type as definition_type,
                    definition.start_line as line_number,
                    definition.primary_file_path as file_path
                ORDER BY definition.start_line
                LIMIT $limit
            "#
            .to_string(),
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

    pub fn get_file_imports_query() -> Query {
        Query {
            query: r#"
                MATCH (file:FileNode)-[:FILE_RELATIONSHIPS]->(imp:ImportedSymbolNode)
                WHERE file.path = $file_path OR file.absolute_path = $file_path
                RETURN
                    imp.name as name,
                    imp.import_path as import_path,
                    imp.alias as alias,
                    imp.start_line as line_number
                LIMIT $limit
            "#
            .to_string(),
            parameters: HashMap::from([
                (
                    "file_path",
                    QueryParameter {
                        name: "file_path",
                        description: "The path of the file to get imports for.",
                        required: true,
                        definition: QueryParameterDefinition::String(None),
                    },
                ),
                (
                    "limit",
                    QueryParameter {
                        name: "limit",
                        description: "The maximum number of imports to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(100)),
                    },
                ),
            ]),
            result: HashMap::from([
                ("name", STRING_MAPPER),
                ("import_path", STRING_MAPPER),
                ("alias", STRING_MAPPER),
                ("line_number", INT_MAPPER),
            ]),
        }
    }

    pub fn get_list_matches_query() -> Query {
        Query {
            query: r#"
                MATCH (n:DefinitionNode)
                WHERE toLower(n.fqn) CONTAINS toLower($search_string)
                RETURN
                    n.fqn as fqn,
                    n.name as name,
                    n.definition_type as definition_type,
                    n.primary_file_path as file_path,
                    n.start_line as line_number
                ORDER BY n.fqn
                LIMIT $limit
            "#
            .to_string(),
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
                    CAST(0 AS INT64) as source_start_line,
                    CAST(0 AS INT64) as source_primary_start_byte,
                    CAST(0 AS INT64) as source_primary_end_byte,
                    CAST(0 AS INT64) as source_total_locations,
                    '' as source_import_type,
                    '' as source_import_path,
                    '' as source_import_alias,
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
                    CAST(0 AS INT64) as target_start_line,
                    CAST(0 AS INT64) as target_primary_start_byte,
                    CAST(0 AS INT64) as target_primary_end_byte,
                    CAST(0 AS INT64) as target_total_locations,
                    '' as target_import_type,
                    '' as target_import_path,
                    '' as target_import_alias,
                    'DIRECTORY_RELATIONSHIPS' as relationship_type,
                    id(r) as relationship_id,
                    1 as order_priority
                LIMIT $directory_limit
                UNION
                MATCH (n:DirectoryNode)-[r:DIRECTORY_RELATIONSHIPS]-(f:FileNode)
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
                    CAST(0 AS INT64) as source_start_line,
                    CAST(0 AS INT64) as source_primary_start_byte,
                    CAST(0 AS INT64) as source_primary_end_byte,
                    CAST(0 AS INT64) as source_total_locations,
                    '' as source_import_type,
                    '' as source_import_path,
                    '' as source_import_alias,
                    f.id as target_id,
                    'FileNode' as target_type,
                    f.name as target_name,
                    f.path as target_path,
                    f.absolute_path as target_absolute_path,
                    f.repository_name as target_repository_name,
                    '' as target_fqn,
                    '' as target_definition_type,
                    f.language as target_language,
                    f.extension as target_extension,
                    CAST(0 AS INT64) as target_start_line,
                    CAST(0 AS INT64) as target_primary_start_byte,
                    CAST(0 AS INT64) as target_primary_end_byte,
                    CAST(0 AS INT64) as target_total_locations,
                    '' as target_import_type,
                    '' as target_import_path,
                    '' as target_import_alias,
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
                    CAST(0 AS INT64) as source_start_line,
                    CAST(0 AS INT64) as source_primary_start_byte,
                    CAST(0 AS INT64) as source_primary_end_byte,
                    CAST(0 AS INT64) as source_total_locations,
                    '' as source_import_type,
                    '' as source_import_path,
                    '' as source_import_alias,
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
                    CAST(d.start_line AS INT64) as target_start_line,
                    d.primary_start_byte as target_primary_start_byte,
                    d.primary_end_byte as target_primary_end_byte,
                    CAST(d.total_locations AS INT64) as target_total_locations,
                    '' as target_import_type,
                    '' as target_import_path,
                    '' as target_import_alias,
                    'FILE_RELATIONSHIPS' as relationship_type,
                    id(r) as relationship_id,
                    2 as order_priority
                LIMIT $file_limit
                UNION
                MATCH (f:FileNode)-[r:FILE_RELATIONSHIPS]-(i:ImportedSymbolNode)
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
                    CAST(0 AS INT64) as source_start_line,
                    CAST(0 AS INT64) as source_primary_start_byte,
                    CAST(0 AS INT64) as source_primary_end_byte,
                    CAST(0 AS INT64) as source_total_locations,
                    '' as source_import_type,
                    '' as source_import_path,
                    '' as source_import_alias,
                    i.id as target_id,
                    'ImportedSymbolNode' as target_type,
                    i.name as target_name,
                    i.file_path as target_path,
                    '' as target_absolute_path,
                    '' as target_repository_name,
                    '' as target_fqn,
                    '' as target_definition_type,
                    '' as target_language,
                    '' as target_extension,
                    CAST(i.start_line AS INT64) as target_start_line,
                    i.start_byte as target_primary_start_byte,
                    i.end_byte as target_primary_end_byte,
                    CAST(0 AS INT64) as target_total_locations,
                    i.import_type as target_import_type,
                    i.import_path as target_import_path,
                    i.alias as target_import_alias,
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
                    CAST(d1.start_line AS INT64) as source_start_line,
                    d1.primary_start_byte as source_primary_start_byte,
                    d1.primary_end_byte as source_primary_end_byte,
                    CAST(d1.total_locations AS INT64) as source_total_locations,
                    '' as source_import_type,
                    '' as source_import_path,
                    '' as source_import_alias,
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
                    CAST(d2.start_line AS INT64) as target_start_line,
                    d2.primary_start_byte as target_primary_start_byte,
                    d2.primary_end_byte as target_primary_end_byte,
                    CAST(d2.total_locations AS INT64) as target_total_locations,
                    '' as target_import_type,
                    '' as target_import_path,
                    '' as target_import_alias,
                    'DEFINITION_RELATIONSHIPS' as relationship_type,
                    id(r) as relationship_id,
                    3 as order_priority
                LIMIT $definition_limit
                UNION
                MATCH (d:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]-(i:ImportedSymbolNode)
                RETURN 
                    d.id as source_id,
                    'DefinitionNode' as source_type,
                    d.name as source_name,
                    d.primary_file_path as source_path,
                    '' as source_absolute_path,
                    '' as source_repository_name,
                    d.fqn as source_fqn,
                    d.definition_type as source_definition_type,
                    '' as source_language,
                    '' as source_extension,
                    CAST(d.start_line AS INT64) as source_start_line,
                    d.primary_start_byte as source_primary_start_byte,
                    d.primary_end_byte as source_primary_end_byte,
                    CAST(d.total_locations AS INT64) as source_total_locations,
                    '' as source_import_type,
                    '' as source_import_path,
                    '' as source_import_alias,
                    i.id as target_id,
                    'ImportedSymbolNode' as target_type,
                    i.name as target_name,
                    i.file_path as target_path,
                    '' as target_absolute_path,
                    '' as target_repository_name,
                    '' as target_fqn,
                    '' as target_definition_type,
                    '' as target_language,
                    '' as target_extension,
                    CAST(i.start_line AS INT64) as target_start_line,
                    i.start_byte as target_primary_start_byte,
                    i.end_byte as target_primary_end_byte,
                    CAST(0 AS INT64) as target_total_locations,
                    i.import_type as target_import_type,
                    i.import_path as target_import_path,
                    i.alias as target_import_alias,
                    'DEFINITION_RELATIONSHIPS' as relationship_type,
                    id(r) as relationship_id,
                    3 as order_priority
                LIMIT $imported_symbol_limit
            "#
            .to_string(),
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
                (
                    "imported_symbol_limit",
                    QueryParameter {
                        name: "imported_symbol_limit",
                        description: "The maximum number of imported symbol relationships to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(50)),
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
                ("source_start_line", INT_MAPPER),
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
                ("target_start_line", INT_MAPPER),
                ("target_primary_start_byte", INT_MAPPER),
                ("target_primary_end_byte", INT_MAPPER),
                ("target_total_locations", INT_MAPPER),
                ("relationship_type", RELATIONSHIP_TYPE_MAPPER),
                ("relationship_id", STRING_MAPPER),
                ("order_priority", INT_MAPPER),
                ("import_type", STRING_MAPPER),
                ("import_path", STRING_MAPPER),
                ("import_alias", STRING_MAPPER),
            ]),
        }
    }

    fn get_node_neighbors_return_values(node_type: &str, alias: &str) -> String {
        match node_type {
            "DirectoryNode" => format!(
                r#"
                {alias}.id as {alias}_id, 
                'DirectoryNode' as {alias}_type,
                {alias}.name as {alias}_name,
                {alias}.path as {alias}_path,
                {alias}.absolute_path as {alias}_absolute_path,
                {alias}.repository_name as {alias}_repository_name,
                '' as {alias}_fqn,
                '' as {alias}_definition_type,
                '' as {alias}_language,
                '' as {alias}_extension,
                CAST(0 AS INT64) as {alias}_start_line,
                CAST(0 AS INT64) as {alias}_primary_start_byte,
                CAST(0 AS INT64) as {alias}_primary_end_byte,
                CAST(0 AS INT64) as {alias}_total_locations,
                '' as {alias}_import_type,
                '' as {alias}_import_path,
                '' as {alias}_import_alias,
            "#
            ),
            "FileNode" => format!(
                r#"
                    {alias}.id as {alias}_id,
                    'FileNode' as {alias}_type,
                    {alias}.name as {alias}_name,
                    {alias}.path as {alias}_path,
                    {alias}.absolute_path as {alias}_absolute_path,
                    {alias}.repository_name as {alias}_repository_name,
                    '' as {alias}_fqn,
                    '' as {alias}_definition_type,
                    {alias}.language as {alias}_language,
                    {alias}.extension as {alias}_extension,
                    CAST(0 AS INT64) as {alias}_start_line,
                    CAST(0 AS INT64) as {alias}_primary_start_byte,
                    CAST(0 AS INT64) as {alias}_primary_end_byte,
                    CAST(0 AS INT64) as {alias}_total_locations,
                    '' as {alias}_import_type,
                    '' as {alias}_import_path,
                    '' as {alias}_import_alias,
                "#
            ),
            "DefinitionNode" => format!(
                r#"
                    {alias}.id as {alias}_id,
                    'DefinitionNode' as {alias}_type,
                    {alias}.name as {alias}_name,
                    {alias}.primary_file_path as {alias}_path,
                    '' as {alias}_absolute_path,
                    '' as {alias}_repository_name,
                    {alias}.fqn as {alias}_fqn,
                    {alias}.definition_type as {alias}_definition_type,
                    '' as {alias}_language,
                    '' as {alias}_extension,
                    CAST({alias}.start_line AS INT64) as {alias}_start_line,
                    {alias}.primary_start_byte as {alias}_primary_start_byte,
                    {alias}.primary_end_byte as {alias}_primary_end_byte,
                    CAST({alias}.total_locations AS INT64) as {alias}_total_locations,
                    '' as {alias}_import_type,
                    '' as {alias}_import_path,
                    '' as {alias}_import_alias,
                "#
            ),
            "ImportedSymbolNode" => format!(
                r#"
                    {alias}.id as {alias}_id,
                    'ImportedSymbolNode' as {alias}_type,
                    {alias}.name as {alias}_name,
                    {alias}.file_path as {alias}_path,
                    '' as {alias}_absolute_path,
                    '' as {alias}_repository_name,
                    '' as {alias}_fqn,
                    '' as {alias}_definition_type,
                    '' as {alias}_language,
                    '' as {alias}_extension,
                    CAST({alias}.start_line AS INT64) as {alias}_start_line,
                    {alias}.start_byte as {alias}_primary_start_byte,
                    {alias}.end_byte as {alias}_primary_end_byte,
                    CAST(0 AS INT64) as {alias}_total_locations,
                    {alias}.import_type as {alias}_import_type,
                    {alias}.import_path as {alias}_import_path,
                    {alias}.alias as {alias}_import_alias,
                "#
            ),
            _ => "".to_string(),
        }
    }

    pub fn get_node_neighbors_query(node_type: &str) -> Option<Query> {
        let parts = match node_type {
            "DirectoryNode" => vec![
                (
                    "DirectoryNode",
                    "DIRECTORY_RELATIONSHIPS",
                    "DirectoryNode",
                    "source.id = $node_id",
                ),
                (
                    "DirectoryNode",
                    "DIRECTORY_RELATIONSHIPS",
                    "FileNode",
                    "source.id = $node_id",
                ),
            ],
            "FileNode" => vec![
                (
                    "DirectoryNode",
                    "DIRECTORY_RELATIONSHIPS",
                    "FileNode",
                    "target.id = $node_id",
                ),
                (
                    "FileNode",
                    "FILE_RELATIONSHIPS",
                    "DefinitionNode",
                    "source.id = $node_id",
                ),
                (
                    "FileNode",
                    "FILE_RELATIONSHIPS",
                    "ImportedSymbolNode",
                    "source.id = $node_id",
                ),
            ],
            "DefinitionNode" => vec![
                (
                    "FileNode",
                    "FILE_RELATIONSHIPS",
                    "DefinitionNode",
                    "target.id = $node_id",
                ),
                (
                    "DefinitionNode",
                    "DEFINITION_RELATIONSHIPS",
                    "DefinitionNode",
                    "source.id = $node_id",
                ),
                (
                    "DefinitionNode",
                    "DEFINITION_RELATIONSHIPS",
                    "ImportedSymbolNode",
                    "source.id = $node_id",
                ),
            ],
            "ImportedSymbolNode" => vec![
                (
                    "FileNode",
                    "FILE_RELATIONSHIPS",
                    "ImportedSymbolNode",
                    "target.id = $node_id",
                ),
                (
                    "DefinitionNode",
                    "DEFINITION_RELATIONSHIPS",
                    "ImportedSymbolNode",
                    "target.id = $node_id",
                ),
            ],
            _ => return None,
        };

        let query = parts.iter().map(|(
            source_type,
            relationship_type,
            target_type,
            condition,
        )| {
            let source_return = Self::get_node_neighbors_return_values(source_type, "source");
            let target_return = Self::get_node_neighbors_return_values(target_type, "target");

            format!(
                r#"
                MATCH (source:{source_type})-[r:{relationship_type}]-(target:{target_type}) WHERE {condition}
                RETURN 
                    {source_return}
                    {target_return}
                    '{relationship_type}' as relationship_type,
                    id(r) as relationship_id
                "#, 
            )
        }).collect::<Vec<String>>().join("\nUNION\n");

        let query = format!("{query} LIMIT $limit");

        Some(Query {
            query,
            parameters: HashMap::from([
                (
                    "node_id",
                    QueryParameter {
                        name: "node_id",
                        description: "The ID of the node to get neighbors for.",
                        required: true,
                        definition: QueryParameterDefinition::String(None),
                    },
                ),
                (
                    "limit",
                    QueryParameter {
                        name: "limit",
                        description: "The maximum number of neighbors to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(100)),
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
                ("source_start_line", INT_MAPPER),
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
                ("target_start_line", INT_MAPPER),
                ("target_primary_start_byte", INT_MAPPER),
                ("target_primary_end_byte", INT_MAPPER),
                ("target_total_locations", INT_MAPPER),
                ("relationship_type", RELATIONSHIP_TYPE_MAPPER),
                ("relationship_id", STRING_MAPPER),
            ]),
        })
    }

    pub fn get_search_nodes_query() -> Query {
        Query {
            query: r#"
                MATCH (d:DirectoryNode)
                WHERE toLower(d.name) CONTAINS toLower($search_term) 
                   OR toLower(d.path) CONTAINS toLower($search_term)
                RETURN 
                    d.id as id,
                    'DirectoryNode' as node_type,
                    d.name as name,
                    d.path as path,
                    d.absolute_path as absolute_path,
                    d.repository_name as repository_name,
                    '' as fqn,
                    '' as definition_type,
                    '' as language,
                    '' as extension,
                    CAST(0 AS INT64) as start_line,
                    CAST(0 AS INT64) as primary_start_byte,
                    CAST(0 AS INT64) as primary_end_byte,
                    CAST(0 AS INT64) as total_locations,
                    '' as import_type,
                    '' as import_path,
                    '' as import_alias
                UNION
                MATCH (f:FileNode)
                WHERE toLower(f.name) CONTAINS toLower($search_term)
                   OR toLower(f.path) CONTAINS toLower($search_term)
                RETURN 
                    f.id as id,
                    'FileNode' as node_type,
                    f.name as name,
                    f.path as path,
                    f.absolute_path as absolute_path,
                    f.repository_name as repository_name,
                    '' as fqn,
                    '' as definition_type,
                    f.language as language,
                    f.extension as extension,
                    CAST(0 AS INT64) as start_line,
                    CAST(0 AS INT64) as primary_start_byte,
                    CAST(0 AS INT64) as primary_end_byte,
                    CAST(0 AS INT64) as total_locations,
                    '' as import_type,
                    '' as import_path,
                    '' as import_alias
                UNION
                MATCH (def:DefinitionNode)
                WHERE toLower(def.name) CONTAINS toLower($search_term)
                   OR toLower(def.fqn) CONTAINS toLower($search_term)
                RETURN 
                    def.id as id,
                    'DefinitionNode' as node_type,
                    def.name as name,
                    def.primary_file_path as path,
                    '' as absolute_path,
                    '' as repository_name,
                    def.fqn as fqn,
                    def.definition_type as definition_type,
                    '' as language,
                    '' as extension,
                    CAST(def.start_line AS INT64) as start_line,
                    def.primary_start_byte as primary_start_byte,
                    def.primary_end_byte as primary_end_byte,
                    CAST(def.total_locations AS INT64) as total_locations,
                    '' as import_type,
                    '' as import_path,
                    '' as import_alias
                UNION
                MATCH (imp:ImportedSymbolNode)
                WHERE toLower(imp.name) CONTAINS toLower($search_term)
                   OR toLower(imp.import_path) CONTAINS toLower($search_term)
                   OR toLower(imp.alias) CONTAINS toLower($search_term)
                RETURN 
                    imp.id as id,
                    'ImportedSymbolNode' as node_type,
                    imp.name as name,
                    imp.file_path as path,
                    '' as absolute_path,
                    '' as repository_name,
                    '' as fqn,
                    '' as definition_type,
                    '' as language,
                    '' as extension,
                    CAST(imp.start_line AS INT64) as start_line,
                    imp.start_byte as primary_start_byte,
                    imp.end_byte as primary_end_byte,
                    CAST(0 AS INT64) as total_locations,
                    imp.import_type as import_type,
                    imp.import_path as import_path,
                    imp.alias as import_alias
                ORDER BY node_type, name
                LIMIT $limit
            "#
            .to_string(),
            parameters: HashMap::from([
                (
                    "search_term",
                    QueryParameter {
                        name: "search_term",
                        description: "The search term to match against node names, paths, or FQNs (case insensitive).",
                        required: true,
                        definition: QueryParameterDefinition::String(None),
                    },
                ),
                (
                    "limit",
                    QueryParameter {
                        name: "limit",
                        description: "The maximum number of search results to return.",
                        required: false,
                        definition: QueryParameterDefinition::Int(Some(100)),
                    },
                ),
            ]),
            result: HashMap::from([
                ("id", STRING_MAPPER),
                ("node_type", STRING_MAPPER),
                ("name", STRING_MAPPER),
                ("path", STRING_MAPPER),
                ("absolute_path", STRING_MAPPER),
                ("repository_name", STRING_MAPPER),
                ("fqn", STRING_MAPPER),
                ("definition_type", STRING_MAPPER),
                ("language", STRING_MAPPER),
                ("extension", STRING_MAPPER),
                ("start_line", INT_MAPPER),
                ("primary_start_byte", INT_MAPPER),
                ("primary_end_byte", INT_MAPPER),
                ("total_locations", INT_MAPPER),
                ("import_type", STRING_MAPPER),
                ("import_path", STRING_MAPPER),
                ("import_alias", STRING_MAPPER),
            ]),
        }
    }

    pub fn get_definitions_by_fqn_or_name_query() -> Query {
        Query {
            query: r#"
                MATCH (d:DefinitionNode) 
                WHERE 
                    d.primary_file_path = $file_path 
                    AND (
                        toLower(d.name) CONTAINS toLower($name_or_fqn) 
                        OR toLower(d.fqn) CONTAINS toLower($name_or_fqn)
                    ) 
                RETURN 
                    d.id as id,
                    d.name as name,
                    d.fqn as fqn,
                    d.primary_file_path as file_path,
                    d.start_line as line_number
            "#
            .to_string(),
            parameters: HashMap::from([
                (
                    "file_path",
                    QueryParameter {
                        name: "file_path",
                        description: "The path of the file to get the node for.",
                        required: true,
                        definition: QueryParameterDefinition::String(None),
                    },
                ),
                (
                    "name_or_fqn",
                    QueryParameter {
                        name: "name_or_fqn",
                        description: "The name or FQN of the node to get.",
                        required: true,
                        definition: QueryParameterDefinition::String(None),
                    },
                ),
            ]),
            result: HashMap::from([
                ("id", STRING_MAPPER),
                ("name", STRING_MAPPER),
                ("fqn", STRING_MAPPER),
                ("line_number", INT_MAPPER),
                ("file_path", STRING_MAPPER),
            ]),
        }
    }
}
