use thiserror::Error;

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
