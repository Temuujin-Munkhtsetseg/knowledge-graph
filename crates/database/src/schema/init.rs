use crate::schema::types::{ColumnDefinition, NodeTable, RelationshipTable};
use std::sync::LazyLock;

// Directory nodes
pub static DIRECTORY_TABLE: LazyLock<NodeTable> = LazyLock::new(|| NodeTable {
    name: "DirectoryNode",
    parquet_filename: "directories.parquet",
    columns: vec![
        ColumnDefinition::new("id").uint32().primary_key(),
        ColumnDefinition::new("path"),
        ColumnDefinition::new("absolute_path"),
        ColumnDefinition::new("repository_name"),
        ColumnDefinition::new("name"),
    ],
    primary_key: "id",
});

// File nodes
pub static FILE_TABLE: LazyLock<NodeTable> = LazyLock::new(|| NodeTable {
    name: "FileNode",
    parquet_filename: "files.parquet",
    columns: vec![
        ColumnDefinition::new("id").uint32().primary_key(),
        ColumnDefinition::new("path"),
        ColumnDefinition::new("absolute_path"),
        ColumnDefinition::new("language"),
        ColumnDefinition::new("repository_name"),
        ColumnDefinition::new("extension"),
        ColumnDefinition::new("name"),
    ],
    primary_key: "id",
});

// Definition nodes
pub static DEFINITION_TABLE: LazyLock<NodeTable> = LazyLock::new(|| NodeTable {
    name: "DefinitionNode",
    parquet_filename: "definitions.parquet",
    columns: vec![
        ColumnDefinition::new("id").uint32().primary_key(),
        ColumnDefinition::new("fqn"),
        ColumnDefinition::new("name"),
        ColumnDefinition::new("definition_type"),
        ColumnDefinition::new("primary_file_path"),
        ColumnDefinition::new("primary_start_byte").int64(),
        ColumnDefinition::new("primary_end_byte").int64(),
        ColumnDefinition::new("start_line").int32(),
        ColumnDefinition::new("end_line").int32(),
        ColumnDefinition::new("start_col").int32(),
        ColumnDefinition::new("end_col").int32(),
        ColumnDefinition::new("total_locations").int32(),
    ],
    primary_key: "id",
});

// Imported symbol nodes
pub static IMPORTED_SYMBOL_TABLE: LazyLock<NodeTable> = LazyLock::new(|| NodeTable {
    name: "ImportedSymbolNode",
    parquet_filename: "imported_symbols.parquet",
    columns: vec![
        ColumnDefinition::new("id").uint32().primary_key(),
        ColumnDefinition::new("import_type"),
        ColumnDefinition::new("import_path"),
        ColumnDefinition::new("name"),
        ColumnDefinition::new("alias"),
        ColumnDefinition::new("file_path"),
        ColumnDefinition::new("start_byte").int64(),
        ColumnDefinition::new("end_byte").int64(),
        ColumnDefinition::new("start_line").int32(),
        ColumnDefinition::new("end_line").int32(),
        ColumnDefinition::new("start_col").int32(),
        ColumnDefinition::new("end_col").int32(),
    ],
    primary_key: "id",
});

// Node tables
pub static NODE_TABLES: LazyLock<Vec<NodeTable>> = LazyLock::new(|| {
    vec![
        DIRECTORY_TABLE.clone(),
        FILE_TABLE.clone(),
        DEFINITION_TABLE.clone(),
        IMPORTED_SYMBOL_TABLE.clone(),
    ]
});

// Directory relationships (DIR_CONTAINS_DIR + DIR_CONTAINS_FILE)
// Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
pub static DIRECTORY_RELATIONSHIPS: LazyLock<RelationshipTable> =
    LazyLock::new(|| RelationshipTable {
        name: "DIRECTORY_RELATIONSHIPS",
        columns: vec![ColumnDefinition::new("type").uint8()],
        from_to_pairs: vec![
            (&DIRECTORY_TABLE, &DIRECTORY_TABLE),
            (&DIRECTORY_TABLE, &FILE_TABLE),
        ],
    });

// File relationships (FILE_DEFINES + FILE_IMPORTS)
// Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
pub static FILE_RELATIONSHIPS: LazyLock<RelationshipTable> = LazyLock::new(|| RelationshipTable {
    name: "FILE_RELATIONSHIPS",
    columns: vec![
        ColumnDefinition::new("type").uint8(),
        // Optional source location fields for imports and calls
        ColumnDefinition::new("source_start_byte")
            .int64()
            .nullable(),
        ColumnDefinition::new("source_end_byte").int64().nullable(),
        ColumnDefinition::new("source_start_line")
            .int32()
            .nullable(),
        ColumnDefinition::new("source_end_line").int32().nullable(),
        ColumnDefinition::new("source_start_col").int32().nullable(),
        ColumnDefinition::new("source_end_col").int32().nullable(),
    ],
    from_to_pairs: vec![
        (&FILE_TABLE, &DEFINITION_TABLE),
        (&FILE_TABLE, &IMPORTED_SYMBOL_TABLE),
    ],
});

// Definition relationships (DEFINES_IMPORTED_SYMBOL, all MODULE_TO_*, CLASS_TO_*, METHOD_*)
// Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
pub static DEFINITION_RELATIONSHIPS: LazyLock<RelationshipTable> =
    LazyLock::new(|| RelationshipTable {
        name: "DEFINITION_RELATIONSHIPS",
        columns: vec![
            ColumnDefinition::new("type").uint8(),
            // Optional source location fields for import call sites and definition references
            ColumnDefinition::new("source_start_byte")
                .int64()
                .nullable(),
            ColumnDefinition::new("source_end_byte").int64().nullable(),
            ColumnDefinition::new("source_start_line")
                .int32()
                .nullable(),
            ColumnDefinition::new("source_end_line").int32().nullable(),
            ColumnDefinition::new("source_start_col").int32().nullable(),
            ColumnDefinition::new("source_end_col").int32().nullable(),
        ],
        from_to_pairs: vec![
            (&DEFINITION_TABLE, &DEFINITION_TABLE),
            (&DEFINITION_TABLE, &IMPORTED_SYMBOL_TABLE),
        ],
    });

pub static RELATIONSHIP_TABLES: LazyLock<Vec<RelationshipTable>> = LazyLock::new(|| {
    vec![
        DIRECTORY_RELATIONSHIPS.clone(),
        FILE_RELATIONSHIPS.clone(),
        DEFINITION_RELATIONSHIPS.clone(),
    ]
});
