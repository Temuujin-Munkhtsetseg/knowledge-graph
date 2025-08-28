use anyhow::Result;
use database::kuzu::{config::DatabaseConfig, connection::KuzuConnection, database::KuzuDatabase};
use std::sync::Arc;
use tracing::info;
use workspace_manager::WorkspaceManager;

pub struct QueryArgs {
    pub project: String,
    pub query_or_file: String,
}

#[cfg(any(debug_assertions, feature = "dev-tools"))]
pub fn run(
    workspace_manager: Arc<WorkspaceManager>,
    database: Arc<KuzuDatabase>,
    args: QueryArgs,
) -> Result<()> {
    // Get the database path from the project path
    let all_projects = workspace_manager.list_all_projects();
    let project_info = all_projects
        .iter()
        .find(|p| p.project_path == args.project)
        .ok_or_else(|| anyhow::anyhow!("Project not found"))?;
    let db_path = project_info
        .database_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert database path to string"))?;

    // Get the database struct, so we can create a connection to it
    let config = DatabaseConfig::default().read_only();
    let database = database
        .get_or_create_database(db_path, Some(config))
        .ok_or_else(|| anyhow::anyhow!("Failed to create database"))?;

    // Read the query from the file if provided
    let query = if std::path::Path::new(&args.query_or_file).exists() {
        // It's a file path
        std::fs::read_to_string(&args.query_or_file)
            .map_err(|e| anyhow::anyhow!("Failed to read query file: {}", e))?
    } else {
        // It's a query string
        args.query_or_file
    };

    if query.is_empty() {
        anyhow::bail!("Empty query provided");
    }

    // Create a connection to the database and execute the query
    match KuzuConnection::new(&database) {
        Ok(connection) => {
            info!("Connection created successfully");
            match connection.query(&query) {
                Ok(query_result) => {
                    for row in query_result.into_iter() {
                        info!("Row: {:?}", row);
                    }
                }
                Err(e) => anyhow::bail!("Failed to execute query: {:?}", e),
            }
        }
        Err(e) => anyhow::bail!("Failed to create connection to database: {:?}", e),
    }

    Ok(())
}

#[cfg(not(any(debug_assertions, feature = "dev-tools")))]
pub fn run(
    _workspace_manager: Arc<WorkspaceManager>,
    _database: Arc<KuzuDatabase>,
    _args: QueryArgs,
) -> Result<()> {
    anyhow::bail!("Query command is not available. Use --features dev-tools to enable.")
}
