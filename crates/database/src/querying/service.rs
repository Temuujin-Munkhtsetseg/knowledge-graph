use crate::{
    kuzu::{connection::KuzuConnection, database::KuzuDatabase},
    querying::types::{QueryResult, QueryResultRow, QueryingService},
};
use anyhow::{Error, Result};
use serde_json::Map;
use std::{path::PathBuf, sync::Arc};

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

    fn get_int_value(&self, index: usize) -> Result<i64, Error> {
        match &self.row[index] {
            kuzu::Value::Int64(value) => Ok(*value),
            kuzu::Value::Int32(value) => Ok((*value).into()),
            kuzu::Value::Int16(value) => Ok((*value).into()),
            kuzu::Value::Int8(value) => Ok((*value).into()),
            _ => Err(Error::msg(format!(
                "Expected integer value, got: {:?}",
                self.row[index]
            ))),
        }
    }

    fn get_uint_value(&self, index: usize) -> Result<u64, Error> {
        match &self.row[index] {
            kuzu::Value::UInt64(value) => Ok(*value),
            kuzu::Value::UInt32(value) => Ok((*value).into()),
            kuzu::Value::UInt16(value) => Ok((*value).into()),
            kuzu::Value::UInt8(value) => Ok((*value).into()),
            _ => Err(Error::msg(format!(
                "Expected unsigned integer value, got: {:?}",
                self.row[index]
            ))),
        }
    }

    fn count(&self) -> usize {
        self.row.len()
    }
}
pub struct DatabaseQueryingService {
    database: Arc<KuzuDatabase>,
}

/// This service should only be used for uncontrolled query execution (e.g., MCP, Playground, API endpoints).
/// For controlled query execution with strict typing for arguments and return types, a proper service should be created instead.
impl DatabaseQueryingService {
    pub fn new(database: Arc<KuzuDatabase>) -> Self {
        Self { database }
    }
}

impl QueryingService for DatabaseQueryingService {
    fn execute_query(
        &self,
        database_path: PathBuf,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<Box<dyn QueryResult>, Error> {
        let database = self
            .database
            .get_or_create_database(database_path.to_str().unwrap());
        if database.is_none() {
            return Err(Error::msg(format!(
                "Database not found for path: {database_path:?}"
            )));
        }

        let database = database.unwrap();
        let connection = KuzuConnection::new(&database);
        if connection.is_err() {
            return Err(Error::msg(format!(
                "Failed to create connection to database: {database_path:?}"
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
