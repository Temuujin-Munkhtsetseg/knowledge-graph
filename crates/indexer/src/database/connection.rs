use crate::database::DatabaseConfig;
use crate::database::types::*;
use crate::database::utils::{RelationshipType, RelationshipTypeMapping};
use kuzu::{Connection, Database, QueryResult, SystemConfig};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Kuzu error: {0}")]
    Kuzu(#[from] kuzu::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to execute query: {query}. Error: {error}")]
    QueryExecutionError { query: String, error: kuzu::Error },
    #[error("Failed to check existing schema state: {0}")]
    SchemaCheckFailed(kuzu::Error),
    #[error("Database initialization failed: {0}")]
    InitializationFailed(String),
    #[error("Connection closed or invalid")]
    ConnectionClosed,
    #[error("Prepared statement error: {0}")]
    PreparedStatementError(String),
}

pub type DbResult<T> = Result<T, DatabaseError>;

/// Manages Kuzu database connections and operations
pub struct KuzuConnection<'db> {
    connection: Connection<'db>,
    database_path: String,
}

impl<'db> KuzuConnection<'db> {
    /// Create a new Kuzu database connection
    pub fn new(database: &'db Database, database_path: String) -> DbResult<Self> {
        info!("Creating Kuzu database connection");
        let connection = Connection::new(database)?;
        info!("Successfully created Kuzu database connection");

        Ok(Self {
            connection,
            database_path,
        })
    }

    /// Create a database from configuration
    pub fn create_database(config: DatabaseConfig) -> DbResult<Database> {
        info!(
            "Creating Kuzu database from config at: {}",
            config.database_path
        );

        // Delete existing database if it exists to start fresh
        let db_path = Path::new(&config.database_path);
        if db_path.exists() {
            info!("Removing existing database at: {}", config.database_path);
            if db_path.is_dir() {
                std::fs::remove_dir_all(db_path).map_err(|e| {
                    DatabaseError::InitializationFailed(format!(
                        "Failed to remove existing database directory: {e}"
                    ))
                })?;
            } else {
                std::fs::remove_file(db_path).map_err(|e| {
                    DatabaseError::InitializationFailed(format!(
                        "Failed to remove existing database file: {e}"
                    ))
                })?;
            }
        }

        // Create database directory if it doesn't exist
        if let Some(parent) = Path::new(&config.database_path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    DatabaseError::InitializationFailed(format!(
                        "Failed to create database directory: {e}"
                    ))
                })?;
            }
        }

        // Configure system settings
        let mut system_config = SystemConfig::default();

        if let Some(buffer_size) = config.buffer_pool_size {
            system_config = system_config.buffer_pool_size(buffer_size as u64);
        }

        if let Some(compression) = config.enable_compression {
            system_config = system_config.enable_compression(compression);
        }

        if let Some(read_only) = config.read_only {
            system_config = system_config.read_only(read_only);
        }

        if let Some(max_size) = config.max_db_size {
            system_config = system_config.max_db_size(max_size as u64);
        }

        // Create database
        let database = Database::new(&config.database_path, system_config)?;
        info!("Successfully created Kuzu database");

        Ok(database)
    }

    /// Execute a query and return the result
    pub fn query(&self, query: &str) -> DbResult<QueryResult> {
        debug!("Executing query: {}", query);

        self.connection
            .query(query)
            .map_err(|e| DatabaseError::QueryExecutionError {
                query: query.to_string(),
                error: e,
            })
    }

    /// Prepare a statement for repeated execution
    pub fn prepare(&self, query: &str) -> DbResult<kuzu::PreparedStatement> {
        debug!("Preparing statement: {}", query);

        self.connection.prepare(query).map_err(|e| {
            DatabaseError::PreparedStatementError(format!(
                "Failed to prepare statement '{query}': {e}"
            ))
        })
    }

    /// Execute a prepared statement with parameters
    pub fn execute(
        &self,
        statement: &mut kuzu::PreparedStatement,
        params: Vec<(&str, kuzu::Value)>,
    ) -> DbResult<QueryResult> {
        debug!(
            "Executing prepared statement with {} parameters",
            params.len()
        );

        self.connection
            .execute(statement, params)
            .map_err(DatabaseError::Kuzu)
    }

    /// Execute a query that doesn't return results (DDL, DML)
    pub fn execute_ddl(&self, query: &str) -> DbResult<()> {
        debug!("Executing DDL: {}", query);

        let mut result = self.query(query)?;

        // Consume the result to ensure the query executed
        while result.next().is_some() {
            // DDL queries typically don't return data, but we consume any results
        }

        Ok(())
    }

    /// Check if a table exists in the database
    pub fn table_exists(&self, table_name: &str) -> DbResult<bool> {
        let query = "CALL SHOW_TABLES() RETURN *";
        let result = self.query(query)?;

        for row in result {
            if let Some(kuzu::Value::String(existing_table_name)) = row.get(1) {
                // Index 1 contains the table name
                if existing_table_name.eq_ignore_ascii_case(table_name) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get all existing table names
    pub fn get_table_names(&self) -> DbResult<Vec<String>> {
        let query = "CALL SHOW_TABLES() RETURN *";
        let result = self.query(query)?;
        let mut table_names = Vec::new();

        for row in result {
            if let Some(kuzu::Value::String(table_name)) = row.get(1) {
                // Index 1 contains the table name
                table_names.push(table_name.to_string());
            }
        }

        Ok(table_names)
    }

    /// Bulk import data from a Parquet file
    pub fn copy_from_parquet(&self, table_name: &str, file_path: &str) -> DbResult<()> {
        let absolute_path = std::path::Path::new(file_path)
            .canonicalize()
            .map_err(DatabaseError::Io)?;

        // For Parquet files, we don't need HEADER option as schema is embedded
        // Schema information is stored in Parquet metadata
        let query = format!("COPY {} FROM '{}'", table_name, absolute_path.display());

        info!("Importing data into {}: {}", table_name, file_path);
        self.execute_ddl(&query)?;
        info!("Successfully imported data into {}", table_name);

        Ok(())
    }

    /// Bulk import relationships from a Parquet file with specific FROM/TO types
    pub fn copy_relationship_from_parquet(
        &self,
        table_name: &str,
        file_path: &str,
        from_table: Option<&str>,
        to_table: Option<&str>,
    ) -> DbResult<()> {
        let absolute_path = std::path::Path::new(file_path)
            .canonicalize()
            .map_err(DatabaseError::Io)?;

        let mut query = format!("COPY {} FROM '{}'", table_name, absolute_path.display());

        // For Parquet files, only from/to options are needed for relationship tables
        // HEADER is not needed as schema is embedded in Parquet metadata
        let mut options = Vec::new();
        if let Some(from) = from_table {
            options.push(format!("from='{from}'"));
        }

        if let Some(to) = to_table {
            options.push(format!("to='{to}'"));
        }

        if !options.is_empty() {
            let options_str: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
            query.push_str(&format!(" ({})", options_str.join(", ")));
        }

        info!(
            "Importing relationship data into {}: {}",
            table_name, file_path
        );
        self.execute_ddl(&query)?;
        info!(
            "Successfully imported relationship data into {}",
            table_name
        );

        Ok(())
    }

    /// Get database statistics
    pub fn get_database_stats(&self) -> DbResult<DatabaseStats> {
        let table_names = self.get_table_names()?;
        let mut node_tables = 0;
        let mut rel_tables = 0;
        let mut total_nodes = 0;
        let mut total_relationships = 0;

        for table_name in &table_names {
            // Check if it's a node or relationship table by trying to count
            let count_query = format!("MATCH (n:{table_name}) RETURN count(n)");
            if let Ok(mut result) = self.query(&count_query) {
                if let Some(row) = result.next() {
                    if let Some(kuzu::Value::Int64(count)) = row.first() {
                        total_nodes += count;
                        node_tables += 1;
                    }
                }
            } else {
                // Try as relationship table
                let rel_count_query = format!("MATCH ()-[r:{table_name}]-() RETURN count(r)");
                if let Ok(mut result) = self.query(&rel_count_query) {
                    if let Some(row) = result.next() {
                        if let Some(kuzu::Value::Int64(count)) = row.first() {
                            total_relationships += count;
                            rel_tables += 1;
                        }
                    }
                }
            }
        }

        Ok(DatabaseStats {
            total_tables: table_names.len(),
            node_tables,
            rel_tables,
            total_nodes: total_nodes as usize,
            total_relationships: total_relationships as usize,
        })
    }

    /// Get the database path
    pub fn database_path(&self) -> &str {
        &self.database_path
    }

    /// Set query timeout (in milliseconds)
    pub fn set_query_timeout(&self, timeout_ms: u64) {
        self.connection.set_query_timeout(timeout_ms);
    }

    /// Interrupt currently running queries
    pub fn interrupt(&self) -> DbResult<()> {
        self.connection.interrupt().map_err(DatabaseError::Kuzu)
    }

    // TODO: abstract these both out into a trait, trait object, or just a higher order method that takes a closure

    pub fn count_nodes(&self, node_type: KuzuNodeType) -> i64 {
        let query = format!("MATCH (n:{}) RETURN COUNT(n)", node_type.as_str());
        let mut result = match self.query(&query) {
            Ok(result) => result,
            Err(_) => return 0,
        };

        if let Some(row) = result.next() {
            if let Some(kuzu::Value::Int64(count)) = row.first() {
                *count
            } else {
                0
            }
        } else {
            0
        }
    }

    pub fn count_relationships_of_type(&self, relationship_type: RelationshipType) -> i64 {
        // Get the relationship label based on the type
        let (rel_label, _type_id) = match relationship_type {
            RelationshipType::DirContainsDir | RelationshipType::DirContainsFile => {
                ("DIRECTORY_RELATIONSHIPS", relationship_type.as_str())
            }
            RelationshipType::FileDefines => ("FILE_RELATIONSHIPS", relationship_type.as_str()),
            _ => {
                // All other types are definition relationships
                ("DEFINITION_RELATIONSHIPS", relationship_type.as_str())
            }
        };

        let query = format!(
            "MATCH (from)-[r:{}]->(to) WHERE r.type = {} RETURN COUNT(DISTINCT [from, to])",
            rel_label,
            RelationshipTypeMapping::new().get_type_id(relationship_type)
        );

        let mut result = match self.query(&query) {
            Ok(result) => result,
            Err(_) => return 0,
        };

        if let Some(row) = result.next() {
            if let Some(kuzu::Value::Int64(count)) = row.first() {
                *count
            } else {
                0
            }
        } else {
            0
        }
    }

    pub fn count_relationships_of_node_type(&self, node_type: KuzuNodeType) -> i64 {
        // Get the relationship label based on the type
        let (rel_label, _type_id) = match node_type {
            KuzuNodeType::DirectoryNode => ("DIRECTORY_RELATIONSHIPS", node_type.as_str()),
            KuzuNodeType::FileNode => ("FILE_RELATIONSHIPS", node_type.as_str()),
            KuzuNodeType::DefinitionNode => ("DEFINITION_RELATIONSHIPS", node_type.as_str()),
        };

        let query = format!("MATCH (from)-[r:{rel_label}]->(to) RETURN COUNT(DISTINCT [from, to])");

        let mut result = match self.query(&query) {
            Ok(result) => result,
            Err(_) => return 0,
        };

        if let Some(row) = result.next() {
            if let Some(kuzu::Value::Int64(count)) = row.first() {
                *count
            } else {
                0
            }
        } else {
            0
        }
    }

    // Knowledge graph specific methods

    pub fn get_all_definition_nodes(&self) -> DbResult<Vec<DefinitionNodeFromKuzu>> {
        let query = "MATCH (n:DefinitionNode) RETURN n";
        let result = self.query(query)?;
        let mut nodes = Vec::new();

        for row in result {
            if let Some(node_value) = row.first() {
                let node = DefinitionNodeFromKuzu::from_kuzu_node(node_value);
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    pub fn get_all_file_nodes(&self) -> DbResult<Vec<FileNodeFromKuzu>> {
        let query = "MATCH (n:FileNode) RETURN n";
        let result = self.query(query)?;
        let mut nodes = Vec::new();

        for row in result {
            if let Some(node_value) = row.first() {
                let node = FileNodeFromKuzu::from_kuzu_node(node_value);
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    pub fn get_all_directory_nodes(&self) -> DbResult<Vec<DirectoryNodeFromKuzu>> {
        let query = "MATCH (n:DirectoryNode) RETURN n";
        let result = self.query(query)?;
        let mut nodes = Vec::new();

        for row in result {
            if let Some(node_value) = row.first() {
                let node = DirectoryNodeFromKuzu::from_kuzu_node(node_value);
                nodes.push(node);
            }
        }
        Ok(nodes)
    }

    pub fn delete_definition_nodes_by_paths(&self, paths: &[String]) -> DbResult<()> {
        let paths_str = paths
            .iter()
            .map(|path| format!("'{path}'"))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "MATCH (n:DefinitionNode) WHERE n.primary_file_path IN [{paths_str}] DETACH DELETE n"
        );

        self.execute_ddl(&query)?;
        Ok(())
    }

    pub fn delete_file_nodes_by_path(&self, paths: &[String]) -> DbResult<()> {
        let paths_str = paths
            .iter()
            .map(|path| format!("'{path}'"))
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!("MATCH (n:FileNode) WHERE n.path IN [{paths_str}] DETACH DELETE n");

        self.execute_ddl(&query)?;
        Ok(())
    }

    pub fn delete_directory_nodes_by_path(&self, paths: &[String]) -> DbResult<()> {
        let paths_str = paths
            .iter()
            .map(|path| format!("'{path}'"))
            .collect::<Vec<_>>()
            .join(", ");

        let query =
            format!("MATCH (n:DirectoryNode) WHERE n.path IN [{paths_str}] DETACH DELETE n");
        self.execute_ddl(&query)?;
        Ok(())
    }

    /// Get all nodes count by type
    pub fn get_node_counts(&self) -> DbResult<NodeCounts> {
        let mut directory_count = 0;
        let mut file_count = 0;
        let mut definition_count = 0;

        // Count directory nodes
        let query = "MATCH (n:DirectoryNode) RETURN count(n)";
        if let Ok(mut result) = self.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    directory_count = *count as u32;
                }
            }
        }

        // Count file nodes
        let query = "MATCH (n:FileNode) RETURN count(n)";
        if let Ok(mut result) = self.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    file_count = *count as u32;
                }
            }
        }

        // Count definition nodes
        let query = "MATCH (n:DefinitionNode) RETURN count(n)";
        if let Ok(mut result) = self.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    definition_count = *count as u32;
                }
            }
        }

        Ok(NodeCounts {
            directory_count,
            file_count,
            definition_count,
        })
    }

    /// Get all relationship counts by type
    pub fn get_relationship_counts(&self) -> DbResult<RelationshipCounts> {
        let mut directory_relationships = 0;
        let mut file_relationships = 0;
        let mut definition_relationships = 0;

        // Count directory relationships
        let query = "MATCH ()-[r:DIRECTORY_RELATIONSHIPS]->() RETURN count(r)";
        if let Ok(mut result) = self.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    directory_relationships = *count as u32;
                }
            }
        }

        // Count file relationships
        let query = "MATCH ()-[r:FILE_RELATIONSHIPS]->() RETURN count(r)";
        if let Ok(mut result) = self.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    file_relationships = *count as u32;
                }
            }
        }

        // Count definition relationships
        let query = "MATCH ()-[r:DEFINITION_RELATIONSHIPS]->() RETURN count(r)";
        if let Ok(mut result) = self.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    definition_relationships = *count as u32;
                }
            }
        }

        Ok(RelationshipCounts {
            directory_relationships,
            file_relationships,
            definition_relationships,
        })
    }
}
