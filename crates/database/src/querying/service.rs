use crate::{
    kuzu::{connection::KuzuConnection, database::KuzuDatabase},
    querying::types::{QueryResult, QueryResultRow, QueryingService},
};
use anyhow::{Error, Result};
use serde_json::Map;
use std::sync::Arc;
use workspace_manager::WorkspaceManager;

struct DatabaseQueryResult {
    column_names: Vec<String>,
    result: Vec<Vec<kuzu::Value>>,
    current_index: usize,
}

impl QueryResult for DatabaseQueryResult {
    fn get_column_names(&self) -> &Vec<String> {
        &self.column_names
    }

    fn next(&mut self) -> Option<Box<dyn QueryResultRow>> {
        if self.current_index >= self.result.len() {
            return None;
        }

        let row = self.result[self.current_index].clone();
        self.current_index += 1;

        Some(Box::new(DatabaseQueryResultRow { row }))
    }
}

struct DatabaseQueryResultRow {
    row: Vec<kuzu::Value>,
}

impl QueryResultRow for DatabaseQueryResultRow {
    fn get_string_value(&self, index: usize) -> Result<String, Error> {
        Ok(self.row[index].to_string())
    }

    fn count(&self) -> usize {
        self.row.len()
    }
}
pub struct DatabaseQueryingService {
    database: Arc<KuzuDatabase>,
    workspace_manager: Arc<WorkspaceManager>,
}

/// This service should only be used for uncontrolled query execution (e.g., MCP, Playground, API endpoints).
/// For controlled query execution with strict typing for arguments and return types, a proper service should be created instead.
impl DatabaseQueryingService {
    pub fn new(database: Arc<KuzuDatabase>, workspace_manager: Arc<WorkspaceManager>) -> Self {
        Self {
            database,
            workspace_manager,
        }
    }
}

impl QueryingService for DatabaseQueryingService {
    fn execute_query(
        &self,
        project_path: &str,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<Box<dyn QueryResult>, Error> {
        let project = self.workspace_manager.get_project_for_path(project_path);

        if project.is_none() {
            return Err(Error::msg(format!(
                "Project not found for path: {project_path}"
            )));
        }

        let database = self.database.get_or_create_database(
            project
                .unwrap()
                .database_path
                .into_os_string()
                .to_str()
                .unwrap(),
        );
        if database.is_none() {
            return Err(Error::msg(format!(
                "Database not found for path: {project_path}"
            )));
        }

        let database = database.unwrap();
        let connection = KuzuConnection::new(&database);
        if connection.is_err() {
            return Err(Error::msg(format!(
                "Failed to create connection to database: {project_path}"
            )));
        }

        let connection = connection.unwrap();

        let result = connection.generic_query(query, params)?;
        Ok(Box::new(DatabaseQueryResult {
            column_names: result.column_names,
            result: result.result,
            current_index: 0,
        }))
    }
}
