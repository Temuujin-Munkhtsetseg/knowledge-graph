use crate::database::{DatabaseError, DbResult, KuzuConnection};
use tracing::{info, warn};

/// Represents a Kuzu node table definition
#[derive(Debug, Clone)]
pub struct NodeTable {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: String,
}

/// Represents a Kuzu relationship table definition
#[derive(Debug, Clone)]
pub struct RelationshipTable {
    pub name: String,
    pub from_table: String,
    pub to_table: String,
    pub columns: Vec<ColumnDefinition>,
}

/// Represents a column definition in a table
#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: KuzuDataType,
    pub is_primary_key: bool,
}

/// Kuzu data types
#[derive(Debug, Clone, PartialEq)]
pub enum KuzuDataType {
    String,
    Int32,
    Int64,
    Float,
    Double,
    Boolean,
    Date,
    Timestamp,
    StringArray,
    Int64Array,
}

impl std::fmt::Display for KuzuDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KuzuDataType::String => write!(f, "STRING"),
            KuzuDataType::Int32 => write!(f, "INT32"),
            KuzuDataType::Int64 => write!(f, "INT64"),
            KuzuDataType::Float => write!(f, "FLOAT"),
            KuzuDataType::Double => write!(f, "DOUBLE"),
            KuzuDataType::Boolean => write!(f, "BOOLEAN"),
            KuzuDataType::Date => write!(f, "DATE"),
            KuzuDataType::Timestamp => write!(f, "TIMESTAMP"),
            KuzuDataType::StringArray => write!(f, "STRING[]"),
            KuzuDataType::Int64Array => write!(f, "INT64[]"),
        }
    }
}

/// Manages database schema creation and operations
pub struct SchemaManager;

impl Default for SchemaManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaManager {
    pub fn new() -> Self {
        Self
    }

    /// Initialize the complete schema for the knowledge graph
    pub fn initialize_schema(&self, connection: &KuzuConnection) -> DbResult<()> {
        info!("Initializing knowledge graph schema...");

        // Check if schema already exists
        if self.schema_exists(connection)? {
            info!("Schema already exists, skipping creation");
            return Ok(());
        }

        // Create node tables
        self.create_node_tables(connection)?;

        // Create relationship tables
        self.create_relationship_tables(connection)?;

        info!("Knowledge graph schema initialized successfully");
        Ok(())
    }

    /// Check if the schema already exists by looking for key tables
    fn schema_exists(&self, connection: &KuzuConnection) -> DbResult<bool> {
        let required_tables = vec!["DirectoryNode", "FileNode", "DefinitionNode"];

        for table in required_tables {
            if !connection.table_exists(table)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Create all node tables
    fn create_node_tables(&self, connection: &KuzuConnection) -> DbResult<()> {
        info!("Creating node tables...");

        // Directory nodes
        let directory_table = NodeTable {
            name: "DirectoryNode".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "path".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: true,
                },
                ColumnDefinition {
                    name: "absolute_path".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "repository_name".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
            ],
            primary_key: "path".to_string(),
        };

        // File nodes
        let file_table = NodeTable {
            name: "FileNode".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "path".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: true,
                },
                ColumnDefinition {
                    name: "absolute_path".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "language".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "repository_name".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "extension".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
            ],
            primary_key: "path".to_string(),
        };

        // Definition nodes (one row per unique FQN)
        // Uses primary location for core data, multiple locations handled separately
        let definition_table = NodeTable {
            name: "DefinitionNode".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "fqn".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: true,
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "definition_type".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "primary_file_path".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "primary_start_byte".to_string(),
                    data_type: KuzuDataType::Int64,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "primary_end_byte".to_string(),
                    data_type: KuzuDataType::Int64,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "primary_line_number".to_string(),
                    data_type: KuzuDataType::Int32,
                    is_primary_key: false,
                },
                ColumnDefinition {
                    name: "total_locations".to_string(),
                    data_type: KuzuDataType::Int32,
                    is_primary_key: false,
                },
            ],
            primary_key: "fqn".to_string(),
        };

        // Create the tables
        self.create_node_table(connection, &directory_table)?;
        self.create_node_table(connection, &file_table)?;
        self.create_node_table(connection, &definition_table)?;

        Ok(())
    }

    /// Create all relationship tables
    fn create_relationship_tables(&self, connection: &KuzuConnection) -> DbResult<()> {
        info!("Creating relationship tables...");

        // Directory contains directory relationship
        let dir_contains_dir = RelationshipTable {
            name: "DIR_CONTAINS_DIR".to_string(),
            from_table: "DirectoryNode".to_string(),
            to_table: "DirectoryNode".to_string(),
            columns: vec![],
        };

        // Directory contains file relationship
        let dir_contains_file = RelationshipTable {
            name: "DIR_CONTAINS_FILE".to_string(),
            from_table: "DirectoryNode".to_string(),
            to_table: "FileNode".to_string(),
            columns: vec![],
        };

        // File defines relationship
        let file_defines = RelationshipTable {
            name: "FILE_DEFINES".to_string(),
            from_table: "FileNode".to_string(),
            to_table: "DefinitionNode".to_string(),
            columns: vec![ColumnDefinition {
                name: "relationship_type".to_string(),
                data_type: KuzuDataType::String,
                is_primary_key: false,
            }],
        };

        // Definition relationships for various types
        let definition_relationships = vec![
            ("MODULE_TO_CLASS", "DefinitionNode", "DefinitionNode"),
            ("MODULE_TO_MODULE", "DefinitionNode", "DefinitionNode"),
            ("MODULE_TO_METHOD", "DefinitionNode", "DefinitionNode"),
            ("CLASS_TO_METHOD", "DefinitionNode", "DefinitionNode"),
            ("CLASS_TO_ATTRIBUTE", "DefinitionNode", "DefinitionNode"),
            ("CLASS_TO_CONSTANT", "DefinitionNode", "DefinitionNode"),
            ("CLASS_INHERITS_FROM", "DefinitionNode", "DefinitionNode"),
            ("METHOD_CALLS", "DefinitionNode", "DefinitionNode"),
            (
                "MODULE_TO_SINGLETON_METHOD",
                "DefinitionNode",
                "DefinitionNode",
            ),
            ("MODULE_TO_CONSTANT", "DefinitionNode", "DefinitionNode"),
            ("MODULE_TO_LAMBDA", "DefinitionNode", "DefinitionNode"),
            ("MODULE_TO_PROC", "DefinitionNode", "DefinitionNode"),
            (
                "CLASS_TO_SINGLETON_METHOD",
                "DefinitionNode",
                "DefinitionNode",
            ),
            ("CLASS_TO_CLASS", "DefinitionNode", "DefinitionNode"),
            ("CLASS_TO_LAMBDA", "DefinitionNode", "DefinitionNode"),
            ("CLASS_TO_PROC", "DefinitionNode", "DefinitionNode"),
            ("METHOD_TO_BLOCK", "DefinitionNode", "DefinitionNode"),
            (
                "SINGLETON_METHOD_TO_BLOCK",
                "DefinitionNode",
                "DefinitionNode",
            ),
        ];

        // Create basic relationship tables
        self.create_relationship_table(connection, &dir_contains_dir)?;
        self.create_relationship_table(connection, &dir_contains_file)?;
        self.create_relationship_table(connection, &file_defines)?;

        // Create definition relationship tables
        for (rel_name, from_table, to_table) in definition_relationships {
            let rel_table = RelationshipTable {
                name: rel_name.to_string(),
                from_table: from_table.to_string(),
                to_table: to_table.to_string(),
                columns: vec![],
            };
            self.create_relationship_table(connection, &rel_table)?;
        }

        Ok(())
    }

    /// Create a single node table
    fn create_node_table(&self, connection: &KuzuConnection, table: &NodeTable) -> DbResult<()> {
        let columns_str = table
            .columns
            .iter()
            .map(|col| {
                let mut col_def = format!("{} {}", col.name, col.data_type);
                if col.is_primary_key {
                    col_def.push_str(" PRIMARY KEY");
                }
                col_def
            })
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "CREATE NODE TABLE IF NOT EXISTS {} ({})",
            table.name, columns_str
        );

        info!("Creating node table: {}", table.name);
        connection.execute_ddl(&query)?;
        info!("Successfully created node table: {}", table.name);

        Ok(())
    }

    /// Create a single relationship table
    fn create_relationship_table(
        &self,
        connection: &KuzuConnection,
        table: &RelationshipTable,
    ) -> DbResult<()> {
        let mut query = format!(
            "CREATE REL TABLE IF NOT EXISTS {} (FROM {} TO {}",
            table.name, table.from_table, table.to_table
        );

        if !table.columns.is_empty() {
            let columns_str = table
                .columns
                .iter()
                .map(|col| format!("{} {}", col.name, col.data_type))
                .collect::<Vec<_>>()
                .join(", ");
            query.push_str(&format!(", {columns_str}"));
        }

        query.push(')');

        info!("Creating relationship table: {}", table.name);
        connection.execute_ddl(&query)?;
        info!("Successfully created relationship table: {}", table.name);

        Ok(())
    }

    /// Import graph data from Parquet files
    pub fn import_graph_data(
        &self,
        connection: &KuzuConnection,
        parquet_dir: &str,
    ) -> DbResult<()> {
        info!(
            "Importing graph data from Parquet files in: {}",
            parquet_dir
        );

        let parquet_path = std::path::Path::new(parquet_dir);
        if !parquet_path.exists() {
            return Err(DatabaseError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Parquet directory not found: {parquet_dir}"),
            )));
        }

        // Import node data
        self.import_nodes(connection, parquet_dir)?;

        // Import relationship data
        self.import_relationships(connection, parquet_dir)?;

        info!("Successfully imported graph data from Parquet files");
        Ok(())
    }

    /// Import node data from Parquet files
    fn import_nodes(&self, connection: &KuzuConnection, parquet_dir: &str) -> DbResult<()> {
        let node_files = vec![
            ("DirectoryNode", "directories.parquet"),
            ("FileNode", "files.parquet"),
            ("DefinitionNode", "definitions.parquet"),
        ];

        for (table_name, file_name) in node_files {
            let file_path = std::path::Path::new(parquet_dir).join(file_name);
            if file_path.exists() {
                info!("Importing {} from {}", table_name, file_path.display());
                connection.copy_from_parquet(table_name, file_path.to_str().unwrap())?;
            } else {
                warn!(
                    "Parquet file not found: {}, skipping import",
                    file_path.display()
                );
            }
        }

        Ok(())
    }

    /// Import relationship data from Parquet files
    fn import_relationships(&self, connection: &KuzuConnection, parquet_dir: &str) -> DbResult<()> {
        let rel_files = vec![
            ("DIR_CONTAINS_DIR", "dir_contains_dir.parquet", None, None),
            ("DIR_CONTAINS_FILE", "dir_contains_file.parquet", None, None),
            (
                "FILE_DEFINES",
                "file_definition_relationships.parquet",
                None,
                None,
            ),
        ];

        for (table_name, file_name, from_table, to_table) in rel_files {
            let file_path = std::path::Path::new(parquet_dir).join(file_name);
            if file_path.exists() {
                info!("Importing {} from {}", table_name, file_path.display());
                connection.copy_relationship_from_parquet(
                    table_name,
                    file_path.to_str().unwrap(),
                    from_table,
                    to_table,
                )?;
            } else {
                warn!(
                    "Parquet file not found: {}, skipping import",
                    file_path.display()
                );
            }
        }

        // Import definition relationships from their individual Parquet files
        let definition_relationship_types = vec![
            "MODULE_TO_CLASS",
            "MODULE_TO_MODULE",
            "MODULE_TO_METHOD",
            "MODULE_TO_SINGLETON_METHOD",
            "MODULE_TO_LAMBDA",
            "MODULE_TO_PROC",
            "CLASS_TO_METHOD",
            "CLASS_TO_SINGLETON_METHOD",
            "CLASS_TO_CLASS",
            "CLASS_TO_LAMBDA",
            "CLASS_TO_PROC",
            "SINGLETON_METHOD_TO_BLOCK",
        ];

        for rel_type in definition_relationship_types {
            let file_name = format!("{}.parquet", rel_type.to_lowercase());
            let file_path = std::path::Path::new(parquet_dir).join(&file_name);
            if file_path.exists() {
                info!("Importing {} from {}", rel_type, file_path.display());
                connection.copy_relationship_from_parquet(
                    rel_type,
                    file_path.to_str().unwrap(),
                    Some("DefinitionNode"),
                    Some("DefinitionNode"),
                )?;
            }
        }

        Ok(())
    }

    /// Get schema statistics
    pub fn get_schema_stats(&self, connection: &KuzuConnection) -> DbResult<SchemaStats> {
        let db_stats = connection.get_database_stats()?;
        let table_names = connection.get_table_names()?;

        Ok(SchemaStats {
            total_tables: db_stats.total_tables,
            node_tables: db_stats.node_tables,
            relationship_tables: db_stats.rel_tables,
            total_nodes: db_stats.total_nodes,
            total_relationships: db_stats.total_relationships,
            table_names,
        })
    }

    /// Run a custom query for analysis
    pub fn query<'a>(
        &self,
        connection: &'a KuzuConnection<'a>,
        query: &str,
    ) -> DbResult<kuzu::QueryResult<'a>> {
        connection.query(query)
    }

    /// Execute a parameterized query
    pub fn execute_query<'a>(
        &self,
        connection: &'a KuzuConnection<'a>,
        query: &str,
        params: Vec<(&str, kuzu::Value)>,
    ) -> DbResult<kuzu::QueryResult<'a>> {
        let mut prepared = connection.prepare(query)?;
        connection.execute(&mut prepared, params)
    }
}

/// Schema statistics
#[derive(Debug, Clone)]
pub struct SchemaStats {
    pub total_tables: usize,
    pub node_tables: usize,
    pub relationship_tables: usize,
    pub total_nodes: usize,
    pub total_relationships: usize,
    pub table_names: Vec<String>,
}

impl std::fmt::Display for SchemaStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Schema Stats: {} tables ({} node, {} rel), {} nodes, {} relationships\nTables: {}",
            self.total_tables,
            self.node_tables,
            self.relationship_tables,
            self.total_nodes,
            self.total_relationships,
            self.table_names.join(", ")
        )
    }
}
