use crate::schema::types::{ColumnDefinition, NodeTable, RelationshipTable};

// Directory nodes
pub static DIRECTORY_TABLE: NodeTable = NodeTable {
    name: "DirectoryNode",
    parquet_filename: "directories.parquet",
    columns: &[
        ColumnDefinition::new("id").uint32().primary_key(),
        ColumnDefinition::new("path"),
        ColumnDefinition::new("absolute_path"),
        ColumnDefinition::new("repository_name"),
        ColumnDefinition::new("name"),
    ],
};

pub static FILE_TABLE: NodeTable = NodeTable {
    name: "FileNode",
    parquet_filename: "files.parquet",
    columns: &[
        ColumnDefinition::new("id").uint32().primary_key(),
        ColumnDefinition::new("path"),
        ColumnDefinition::new("absolute_path"),
        ColumnDefinition::new("language"),
        ColumnDefinition::new("repository_name"),
        ColumnDefinition::new("extension"),
        ColumnDefinition::new("name"),
    ],
};

pub static DEFINITION_TABLE: NodeTable = NodeTable {
    name: "DefinitionNode",
    parquet_filename: "definitions.parquet",
    columns: &[
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
};

// Imported symbol nodes
pub static IMPORTED_SYMBOL_TABLE: NodeTable = NodeTable {
    name: "ImportedSymbolNode",
    parquet_filename: "imported_symbols.parquet",
    columns: &[
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
};

// Node tables
pub static NODE_TABLES: &[NodeTable] = &[
    DIRECTORY_TABLE,
    FILE_TABLE,
    DEFINITION_TABLE,
    IMPORTED_SYMBOL_TABLE,
];

// If we have unused columns, they take up no space by kuzu
// Source id and target id are implicit columns in Kuzu relationships
pub static RELATIONSHIP_TABLE_COLUMNS: &[ColumnDefinition] = &[
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
];

// Directory relationships (DIR_CONTAINS_DIR + DIR_CONTAINS_FILE)
// Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
pub static DIRECTORY_RELATIONSHIPS: RelationshipTable = RelationshipTable {
    name: "DIRECTORY_RELATIONSHIPS",
    columns: RELATIONSHIP_TABLE_COLUMNS,
    from_to_pairs: &[
        (&DIRECTORY_TABLE, &DIRECTORY_TABLE),
        (&DIRECTORY_TABLE, &FILE_TABLE),
    ],
};

// File relationships (FILE_DEFINES + FILE_IMPORTS)
// Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
pub static FILE_RELATIONSHIPS: RelationshipTable = RelationshipTable {
    name: "FILE_RELATIONSHIPS",
    columns: RELATIONSHIP_TABLE_COLUMNS,
    from_to_pairs: &[
        (&FILE_TABLE, &DEFINITION_TABLE),
        (&FILE_TABLE, &IMPORTED_SYMBOL_TABLE),
    ],
};

// Definition relationships (DEFINES_IMPORTED_SYMBOL, all MODULE_TO_*, CLASS_TO_*, METHOD_*)
// Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
pub static DEFINITION_RELATIONSHIPS: RelationshipTable = RelationshipTable {
    name: "DEFINITION_RELATIONSHIPS",
    columns: RELATIONSHIP_TABLE_COLUMNS,
    from_to_pairs: &[
        (&DEFINITION_TABLE, &DEFINITION_TABLE),
        (&DEFINITION_TABLE, &IMPORTED_SYMBOL_TABLE),
    ],
};

// Imported symbol relationships (IMPORTED_SYMBOL_TO_IMPORTED_SYMBOL, IMPORTED_SYMBOL_TO_DEFINITION, IMPORTED_SYMBOL_TO_FILE)
// Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
pub static IMPORTED_SYMBOL_RELATIONSHIPS: RelationshipTable = RelationshipTable {
    name: "IMPORTED_SYMBOL_RELATIONSHIPS",
    columns: RELATIONSHIP_TABLE_COLUMNS,
    from_to_pairs: &[
        (&IMPORTED_SYMBOL_TABLE, &IMPORTED_SYMBOL_TABLE),
        (&IMPORTED_SYMBOL_TABLE, &DEFINITION_TABLE),
        (&IMPORTED_SYMBOL_TABLE, &FILE_TABLE),
    ],
};

pub static RELATIONSHIP_TABLES: &[RelationshipTable] = &[
    DIRECTORY_RELATIONSHIPS,
    FILE_RELATIONSHIPS,
    DEFINITION_RELATIONSHIPS,
    IMPORTED_SYMBOL_RELATIONSHIPS,
];
