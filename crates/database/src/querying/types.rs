use crate::querying::mappers::{QueryResultMapper, STRING_MAPPER};
use anyhow::Error;
use serde_json::Map;
use std::{collections::HashMap, path::PathBuf};

pub trait QueryingService: Send + Sync {
    fn execute_query(
        &self,
        database_path: PathBuf,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<Box<dyn QueryResult>, Error>;
}

pub trait QueryResult: Send + Sync {
    fn get_column_names(&self) -> &Vec<String>;
    fn next(&mut self) -> Option<Box<dyn QueryResultRow>>;
}

impl dyn QueryResult {
    pub fn to_json(
        &mut self,
        result_mappers: &HashMap<&'static str, QueryResultMapper>,
    ) -> Result<serde_json::Value, Error> {
        let mut rows = Vec::new();

        while let Some(row) = self.next() {
            let json_row = row.to_json(self.get_column_names(), result_mappers);

            if json_row.is_err() {
                continue;
            }

            rows.push(json_row.unwrap());
        }

        Ok(serde_json::json!(rows))
    }
}

pub trait QueryResultRow: Send + Sync {
    fn get_string_value(&self, index: usize) -> Result<String, Error>;
    fn get_int_value(&self, index: usize) -> Result<i64, Error>;
    fn get_uint_value(&self, index: usize) -> Result<u64, Error>;
    fn count(&self) -> usize;
}

impl dyn QueryResultRow {
    pub fn to_json(
        &self,
        column_names: &[String],
        result_mappers: &HashMap<&'static str, QueryResultMapper>,
    ) -> Result<serde_json::Value, Error> {
        let mut json = serde_json::Map::with_capacity(column_names.len());

        for (i, column_name) in column_names.iter().enumerate().take(self.count()) {
            let map = result_mappers
                .get(column_name.as_str())
                .unwrap_or(&STRING_MAPPER);
            let value = map(self, i);

            if value.is_err() {
                continue;
            }

            json.insert(column_name.clone(), value.unwrap());
        }

        Ok(serde_json::Value::Object(json))
    }
}
