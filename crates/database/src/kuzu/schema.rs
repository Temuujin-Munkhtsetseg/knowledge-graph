use crate::kuzu::connection::KuzuConnection;
use crate::kuzu::types::DatabaseError;
use kuzu::Database;
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
    pub from_table: Option<String>,
    pub to_table: Option<String>,
    pub columns: Vec<ColumnDefinition>,
    pub from_to_pairs: Option<Vec<(String, String)>>,
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
    UInt32,
    UInt8,
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
            KuzuDataType::UInt32 => write!(f, "UINT32"),
            KuzuDataType::UInt8 => write!(f, "UINT8"),
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
pub struct SchemaManager<'a> {
    database: &'a Database,
}

impl<'a> SchemaManager<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    fn get_connection(&self) -> KuzuConnection {
        match KuzuConnection::new(self.database) {
            Ok(connection) => connection,
            Err(e) => panic!("Failed to create database connection: {e}"),
        }
    }

    /// Initialize the complete schema for the knowledge graph, including creating node and relationship tables
    pub fn initialize_schema(&self) -> Result<(), DatabaseError> {
        info!("Initializing knowledge graph schema...");

        if self.schema_exists()? {
            info!("Schema already exists, skipping creation");
            return Ok(());
        }

        self.get_connection().transaction(|conn| {
            self.create_node_tables(conn)
                .expect("Failed to create node tables");
            self.create_relationship_tables(conn)
                .expect("Failed to create relationship tables");
            Ok(())
        })?;

        info!("Knowledge graph schema initialized successfully");
        Ok(())
    }

    /// Check if the schema already exists by looking for key tables
    fn schema_exists(&self) -> Result<bool, DatabaseError> {
        let connection = self.get_connection();
        let required_tables = vec!["DirectoryNode", "FileNode", "DefinitionNode"];

        for table in required_tables {
            if !connection.table_exists(table)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Create all node tables
    fn create_node_tables(&self, transaction_conn: &KuzuConnection) -> Result<(), DatabaseError> {
        info!("Creating node tables...");

        // Directory nodes
        let directory_table = NodeTable {
            name: "DirectoryNode".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: KuzuDataType::UInt32,
                    is_primary_key: true,
                },
                ColumnDefinition {
                    name: "path".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
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
            primary_key: "id".to_string(),
        };

        // File nodes
        let file_table = NodeTable {
            name: "FileNode".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: KuzuDataType::UInt32,
                    is_primary_key: true,
                },
                ColumnDefinition {
                    name: "path".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
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
            primary_key: "id".to_string(),
        };

        // Definition nodes (one row per unique FQN)
        // Uses primary location for core data, multiple locations handled separately
        let definition_table = NodeTable {
            name: "DefinitionNode".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: KuzuDataType::UInt32,
                    is_primary_key: true,
                },
                ColumnDefinition {
                    name: "fqn".to_string(),
                    data_type: KuzuDataType::String,
                    is_primary_key: false,
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
            primary_key: "id".to_string(),
        };

        // Create the tables
        self.create_node_table(transaction_conn, &directory_table)?;
        self.create_node_table(transaction_conn, &file_table)?;
        self.create_node_table(transaction_conn, &definition_table)?;

        Ok(())
    }

    /// Create all relationship tables with consolidated schema
    fn create_relationship_tables(
        &self,
        transaction_conn: &KuzuConnection,
    ) -> Result<(), DatabaseError> {
        info!("Creating consolidated relationship tables...");

        // Directory relationships (DIR_CONTAINS_DIR + DIR_CONTAINS_FILE)
        // Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
        let directory_relationships = RelationshipTable {
            name: "DIRECTORY_RELATIONSHIPS".to_string(),
            from_table: None,
            to_table: None, // Polymorphic: can be DirectoryNode or FileNode
            columns: vec![ColumnDefinition {
                name: "type".to_string(),
                data_type: KuzuDataType::UInt8,
                is_primary_key: false,
            }],
            from_to_pairs: Some(vec![
                ("DirectoryNode".to_string(), "DirectoryNode".to_string()),
                ("DirectoryNode".to_string(), "FileNode".to_string()),
            ]),
        };

        // File relationships (FILE_DEFINES)
        // Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
        let file_relationships = RelationshipTable {
            name: "FILE_RELATIONSHIPS".to_string(),
            from_table: Some("FileNode".to_string()),
            to_table: Some("DefinitionNode".to_string()),
            columns: vec![ColumnDefinition {
                name: "type".to_string(),
                data_type: KuzuDataType::UInt8,
                is_primary_key: false,
            }],
            from_to_pairs: None,
        };

        // Definition relationships (all MODULE_TO_*, CLASS_TO_*, METHOD_*)
        // Note: Kuzu automatically handles FROM-TO connections, we only need custom properties
        let definition_relationships = RelationshipTable {
            name: "DEFINITION_RELATIONSHIPS".to_string(),
            from_table: Some("DefinitionNode".to_string()),
            to_table: Some("DefinitionNode".to_string()),
            columns: vec![ColumnDefinition {
                name: "type".to_string(),
                data_type: KuzuDataType::UInt8,
                is_primary_key: false,
            }],
            from_to_pairs: None,
        };

        // Create consolidated relationship tables
        self.create_relationship_table(transaction_conn, &directory_relationships)?;
        self.create_relationship_table(transaction_conn, &file_relationships)?;
        self.create_relationship_table(transaction_conn, &definition_relationships)?;

        Ok(())
    }

    /// Create a single node table
    fn create_node_table(
        &self,
        transaction_conn: &KuzuConnection,
        table: &NodeTable,
    ) -> Result<(), DatabaseError> {
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
        transaction_conn.execute_ddl(&query)?;
        info!("Successfully created node table: {}", table.name);

        Ok(())
    }

    /// Create a single relationship table
    fn create_relationship_table(
        &self,
        transaction_conn: &KuzuConnection,
        table: &RelationshipTable,
    ) -> Result<(), DatabaseError> {
        let mut query = format!("CREATE REL TABLE IF NOT EXISTS {} (", table.name);

        // Handle multiple FROM-TO pairs if specified
        if let Some(pairs) = &table.from_to_pairs {
            let from_to_clauses = pairs
                .iter()
                .map(|(from, to)| format!("FROM {from} TO {to}"))
                .collect::<Vec<_>>()
                .join(", ");
            query.push_str(&from_to_clauses);
        } else if let (Some(from), Some(to)) = (&table.from_table, &table.to_table) {
            // Use single FROM-TO for backward compatibility
            query.push_str(&format!("FROM {from} TO {to}"));
        } else {
            return Err(DatabaseError::InitializationFailed(format!(
                "RelationshipTable {} must have either from_to_pairs or both from_table and to_table specified",
                table.name
            )));
        }

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
        transaction_conn.execute_ddl(&query)?;
        info!("Successfully created relationship table: {}", table.name);

        Ok(())
    }

    // Verify that the parquet directory exists and is valid, and log the import
    fn _init_import_graph_data(&self, parquet_dir: &str) -> Result<(), DatabaseError> {
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

        Ok(())
    }

    /// Import graph data from Parquet files
    pub fn import_graph_data(&self, parquet_dir: &str) -> Result<(), DatabaseError> {
        self._init_import_graph_data(parquet_dir)?;
        self.import_nodes_and_relationships(parquet_dir, None)?;
        info!("Successfully imported graph data from Parquet files");
        Ok(())
    }

    // Import graph data with an existing connection, this is used for re-indexing and is for preserving transaction guarantees
    pub fn import_graph_data_with_existing_connection(
        &self,
        parquet_dir: &str,
        existing_connection: &mut KuzuConnection,
    ) -> Result<(), DatabaseError> {
        self._init_import_graph_data(parquet_dir)?;
        self.import_nodes_and_relationships(parquet_dir, Some(existing_connection))?;
        info!("Successfully imported graph data from Parquet files");
        Ok(())
    }

    // Import nodes and relationships in a single transaction
    fn import_nodes_and_relationships(
        &self,
        parquet_dir: &str,
        existing_connection: Option<&mut KuzuConnection>,
    ) -> Result<(), DatabaseError> {
        if let Some(connection) = existing_connection {
            self.import_nodes(connection, parquet_dir)?;
            self.import_relationships(connection, parquet_dir)?;
        } else {
            self.get_connection().transaction(|conn| {
                self.import_nodes(conn, parquet_dir)
                    .expect("Failed to import nodes");
                self.import_relationships(conn, parquet_dir)
                    .expect("Failed to import relationships");
                Ok(())
            })?;
        }
        Ok(())
    }

    /// Import node data from Parquet files
    fn import_nodes(
        &self,
        transaction_conn: &KuzuConnection,
        parquet_dir: &str,
    ) -> Result<(), DatabaseError> {
        let node_files = vec![
            ("DirectoryNode", "directories.parquet"),
            ("FileNode", "files.parquet"),
            ("DefinitionNode", "definitions.parquet"),
        ];

        for (table_name, file_name) in node_files {
            let file_path = std::path::Path::new(parquet_dir).join(file_name);
            if file_path.exists() {
                info!("Importing {} from {}", table_name, file_path.display());
                transaction_conn
                    .copy_nodes_from_parquet(table_name, file_path.to_str().unwrap())?;
            } else {
                warn!(
                    "Parquet file not found: {}, skipping import",
                    file_path.display()
                );
            }
        }

        Ok(())
    }

    /// Import consolidated relationship data from Parquet files
    fn import_relationships(
        &self,
        transaction_conn: &KuzuConnection,
        parquet_dir: &str,
    ) -> Result<(), DatabaseError> {
        // Import directory-to-directory relationships
        let dir_to_dir_file =
            std::path::Path::new(parquet_dir).join("directory_to_directory_relationships.parquet");
        if dir_to_dir_file.exists() {
            match transaction_conn.copy_relationships_from_parquet(
                "DIRECTORY_RELATIONSHIPS",
                dir_to_dir_file.to_str().unwrap(),
                Some("DirectoryNode"),
                Some("DirectoryNode"),
            ) {
                Ok(_) => info!(
                    "Successfully imported DIRECTORY_RELATIONSHIPS (DirectoryNode -> DirectoryNode)"
                ),
                Err(e) => warn!(
                    "Failed to import DirectoryNode->DirectoryNode relationships: {}",
                    e
                ),
            }
        }

        // Import directory-to-file relationships
        let dir_to_file_file =
            std::path::Path::new(parquet_dir).join("directory_to_file_relationships.parquet");
        if dir_to_file_file.exists() {
            match transaction_conn.copy_relationships_from_parquet(
                "DIRECTORY_RELATIONSHIPS",
                dir_to_file_file.to_str().unwrap(),
                Some("DirectoryNode"),
                Some("FileNode"),
            ) {
                Ok(_) => info!(
                    "Successfully imported DIRECTORY_RELATIONSHIPS (DirectoryNode -> FileNode)"
                ),
                Err(e) => warn!(
                    "Failed to import DirectoryNode->FileNode relationships: {}",
                    e
                ),
            }
        }

        // Import other relationship files with specific from/to types
        let other_rel_files = vec![
            (
                "FILE_RELATIONSHIPS",
                "file_relationships.parquet",
                Some("FileNode"),
                Some("DefinitionNode"),
            ),
            (
                "DEFINITION_RELATIONSHIPS",
                "definition_relationships.parquet",
                Some("DefinitionNode"),
                Some("DefinitionNode"),
            ),
        ];

        for (table_name, file_name, from_table, to_table) in other_rel_files {
            let file_path = std::path::Path::new(parquet_dir).join(file_name);
            if file_path.exists() {
                info!("Importing {} from {}", table_name, file_path.display());
                match transaction_conn.copy_relationships_from_parquet(
                    table_name,
                    file_path.to_str().unwrap(),
                    from_table,
                    to_table,
                ) {
                    Ok(_) => info!("Successfully imported {}", table_name),
                    Err(e) => {
                        let error_msg = format!("Failed to import {table_name}: {e}");
                        warn!("{}", error_msg);
                        return Err(e);
                    }
                }
            } else {
                // FILE_RELATIONSHIPS and DEFINITION_RELATIONSHIPS are optional
                // They won't exist when there are no definitions found during indexing, e.g due to an unimplemented language or when re-indexing
                info!(
                    "Relationship file not found: {}, skipping import",
                    file_path.display()
                );
            }
        }

        info!("Successfully imported all available consolidated relationship data");
        Ok(())
    }

    /// Get schema statistics
    pub fn get_schema_stats(&self) -> Result<SchemaStats, DatabaseError> {
        let connection = self.get_connection();
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
