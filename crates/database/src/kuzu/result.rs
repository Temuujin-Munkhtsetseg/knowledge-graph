use crate::{DatabaseResult, DatabaseRow};
use anyhow::Error;

pub struct KuzuDatabaseResult {
    column_names: Vec<String>,
    result: Vec<Vec<kuzu::Value>>,
    current_index: usize,
}

impl KuzuDatabaseResult {
    pub fn new(column_names: Vec<String>, result: Vec<Vec<kuzu::Value>>) -> Self {
        Self {
            column_names,
            result,
            current_index: 0,
        }
    }
}

pub struct KuzuDatabaseRow {
    row: Vec<kuzu::Value>,
}

impl DatabaseRow for KuzuDatabaseRow {
    fn get_string_value(&self, index: usize) -> Result<String, Error> {
        Ok(self.row[index].to_string())
    }

    fn count(&self) -> usize {
        self.row.len()
    }
}

impl DatabaseResult for KuzuDatabaseResult {
    fn get_column_names(&self) -> &Vec<String> {
        &self.column_names
    }

    fn next(&mut self) -> Option<Box<dyn DatabaseRow>> {
        if self.current_index >= self.result.len() {
            return None;
        }

        let row = self.result[self.current_index].clone();
        self.current_index += 1;

        Some(Box::new(KuzuDatabaseRow { row }))
    }
}
