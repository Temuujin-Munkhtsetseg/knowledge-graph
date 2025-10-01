use crate::kuzu::connection::KuzuConnection;
use crate::kuzu::types::DatabaseError;
use crate::kuzu::types::QueryNoop;
use crate::querying::query_builder::QueryBuilder;
use crate::schema::init::{NODE_TABLES, RELATIONSHIP_TABLES};
use crate::schema::types::{NodeTable, RelationshipTable, SchemaStats};
use dunce;
use kuzu::Database;
use tracing::{info, warn};

/// Manages database schema creation and operations
pub struct SchemaManager<'a> {
    database: &'a Database,
    query_builder: QueryBuilder,
}

impl<'a> SchemaManager<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self {
            database,
            query_builder: QueryBuilder::new(),
        }
    }

    fn get_connection(&self) -> KuzuConnection<'_> {
        match KuzuConnection::new(self.database) {
            Ok(connection) => connection,
            Err(e) => panic!("Failed to create database connection: {e}"),
        }
    }

    /// Check if the schema already exists by looking for key tables
    fn schema_exists(&self) -> Result<bool, DatabaseError> {
        let connection = self.get_connection();
        for table in NODE_TABLES.iter() {
            if !connection.table_exists(table.name)? {
                return Ok(false);
            }
        }
        for table in RELATIONSHIP_TABLES.iter() {
            if !connection.table_exists(table.name)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Initialize the complete schema for the knowledge graph, including creating node and relationship tables
    pub fn initialize_schema(&self) -> Result<(), DatabaseError> {
        info!("Initializing knowledge graph schema...");

        if self.schema_exists()? {
            info!("Schema already exists, skipping creation");
            return Ok(());
        }

        // Setup node tables and relationship tables in a single transaction
        self.get_connection().transaction(|conn| {
            for table in NODE_TABLES.iter() {
                self.create_node_table(conn, table)?;
            }
            for table in RELATIONSHIP_TABLES.iter() {
                self.create_relationship_table(conn, table)?;
            }
            Ok(())
        })?;

        info!("Knowledge graph schema initialized successfully");
        Ok(())
    }

    /// Create a single node table
    fn create_node_table(
        &self,
        transaction_conn: &KuzuConnection,
        table: &NodeTable,
    ) -> Result<(), DatabaseError> {
        let (_, query) = self.query_builder.create_node_table(table);
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
        if table.from_to_pairs.is_empty() {
            return Err(DatabaseError::InitializationFailed(format!(
                "RelationshipTable {} must have from_to_pairs specified",
                table.name
            )));
        }
        let (noop, query) = self.query_builder.create_relationship_table(table);
        if noop == QueryNoop::Yes {
            return Ok(());
        }

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
        for table in NODE_TABLES.iter() {
            let file_path = std::path::Path::new(parquet_dir).join(table.parquet_filename);
            if file_path.exists() {
                // On Windows, `std::fs::canonicalize` can return a UNC path that is not
                // well-handled by some programs. `dunce::canonicalize` is a drop-in
                // replacement that avoids this issue. On other platforms, it's an
                // alias for `std::fs::canonicalize`.
                let canonical_path = dunce::canonicalize(&file_path).map_err(|e| {
                    DatabaseError::Io(std::io::Error::other(format!(
                        "Failed to canonicalize path {}: {}",
                        file_path.display(),
                        e
                    )))
                })?;
                info!("Importing {} from {}", table.name, canonical_path.display());
                transaction_conn
                    .copy_nodes_from_parquet(table.name, canonical_path.to_str().unwrap())
                    .map_err(|e| {
                        warn!("Failed to import {}: {}", table.name, e);
                        e
                    })?;
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
        for table in RELATIONSHIP_TABLES.iter() {
            for (from, to) in table.from_to_pairs {
                let filename = from.relationship_filename(to);
                let file_path = std::path::Path::new(parquet_dir).join(filename);
                if file_path.exists() {
                    // On Windows, `std::fs::canonicalize` can return a UNC path that is not
                    // well-handled by some programs. `dunce::canonicalize` is a drop-in
                    // replacement that avoids this issue. On other platforms, it's an
                    // alias for `std::fs::canonicalize`.
                    let canonical_path = dunce::canonicalize(&file_path).map_err(|e| {
                        DatabaseError::Io(std::io::Error::other(format!(
                            "Failed to canonicalize path {}: {}",
                            file_path.display(),
                            e
                        )))
                    })?;
                    match transaction_conn.copy_relationships_from_parquet(
                        table.name,
                        canonical_path.to_str().unwrap(),
                        from.name,
                        to.name,
                    ) {
                        Ok(_) => info!(
                            "Successfully imported {} ({} -> {})",
                            table.name, from.name, to.name
                        ),
                        Err(e) => warn!(
                            "Failed to import {} ({} -> {}): {}",
                            table.name, from.name, to.name, e
                        ),
                    }
                } else {
                    warn!(
                        "Parquet file not found for relationship table: {}(path: {}), skipping import",
                        table.name,
                        file_path.display()
                    );
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kuzu::database::KuzuDatabase;

    #[test]
    fn test_schema_stats() -> Result<(), DatabaseError> {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();

        let kuzu_database = KuzuDatabase::new();
        let temp_dir = tempfile::tempdir()?;
        let temp_dir_path = temp_dir.path().to_str().unwrap();
        let dbpath = format!("{}/database.kz", temp_dir_path);
        println!("dbpath: {}", dbpath);

        let database = match kuzu_database.get_or_create_database(&dbpath, None) {
            Some(db) => db,
            None => {
                panic!("Failed to get or create database");
            }
        };

        let schema_manager = SchemaManager::new(&database);
        schema_manager.initialize_schema()?;
        let stats = schema_manager.get_schema_stats().unwrap();
        assert_eq!(stats.total_tables, 8);
        assert_eq!(stats.node_tables, 4);
        assert_eq!(stats.relationship_tables, 4);
        assert_eq!(stats.total_nodes, 0);
        assert_eq!(stats.total_relationships, 0);
        println!("{}", stats);

        Ok(())
    }
}
