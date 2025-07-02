use crate::{DatabaseResult, DatabaseRow};
use anyhow::{Error, anyhow};

pub struct MockDatabaseRow {
    pub values: Vec<String>,
}

impl DatabaseRow for MockDatabaseRow {
    fn get_string_value(&self, index: usize) -> Result<String, Error> {
        self.values
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Index {} out of bounds", index))
    }

    fn count(&self) -> usize {
        self.values.len()
    }
}

pub struct MockDatabaseResult {
    pub column_names: Vec<String>,
    pub rows: std::vec::IntoIter<Box<dyn DatabaseRow>>,
}

impl MockDatabaseResult {
    pub fn new(column_names: Vec<String>, data: Vec<Vec<String>>) -> Self {
        let rows: Vec<Box<dyn DatabaseRow>> = data
            .into_iter()
            .map(|values| Box::new(MockDatabaseRow { values }) as Box<dyn DatabaseRow>)
            .collect();

        Self {
            column_names,
            rows: rows.into_iter(),
        }
    }
}

impl DatabaseResult for MockDatabaseResult {
    fn get_column_names(&self) -> &Vec<String> {
        &self.column_names
    }

    fn next(&mut self) -> Option<Box<dyn DatabaseRow>> {
        self.rows.next()
    }
}
