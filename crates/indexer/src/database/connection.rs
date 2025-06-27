use crate::database::DatabaseConfig;
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
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_tables: usize,
    pub node_tables: usize,
    pub rel_tables: usize,
    pub total_nodes: usize,
    pub total_relationships: usize,
}

impl std::fmt::Display for DatabaseStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Database Stats: {} tables ({} node, {} rel), {} nodes, {} relationships",
            self.total_tables,
            self.node_tables,
            self.rel_tables,
            self.total_nodes,
            self.total_relationships
        )
    }
}
