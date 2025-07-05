use crate::{querying::QueryResult, querying::QueryResultRow, querying::QueryingService};
use anyhow::{Error, anyhow};
use serde_json::{Map, Value};

pub struct MockQueryingService {
    pub should_fail: bool,
    pub expected_project_path: Option<String>,
    pub expected_query: Option<String>,
    pub expected_params: Option<Map<String, Value>>,
    pub return_data: Vec<Vec<String>>,
    pub column_names: Vec<String>,
}

impl Default for MockQueryingService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockQueryingService {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            expected_project_path: None,
            expected_query: None,
            expected_params: None,
            return_data: vec![vec!["test_value".to_string()]],
            column_names: vec!["test_column".to_string()],
        }
    }

    pub fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    pub fn with_expectations(
        mut self,
        project_path: String,
        query: String,
        params: Map<String, Value>,
    ) -> Self {
        self.expected_project_path = Some(project_path);
        self.expected_query = Some(query);
        self.expected_params = Some(params);
        self
    }

    pub fn with_return_data(mut self, column_names: Vec<String>, data: Vec<Vec<String>>) -> Self {
        self.column_names = column_names;
        self.return_data = data;
        self
    }
}

impl QueryingService for MockQueryingService {
    fn execute_query(
        &self,
        project_path: &str,
        query: &str,
        params: Map<String, Value>,
    ) -> Result<Box<dyn QueryResult>, Error> {
        if self.should_fail {
            return Err(anyhow!("Mock query service failure"));
        }

        // Verify expectations if set
        if let Some(expected_path) = &self.expected_project_path {
            assert_eq!(project_path, expected_path, "Project path mismatch");
        }
        if let Some(expected_query) = &self.expected_query {
            assert_eq!(query, expected_query, "Query mismatch");
        }
        if let Some(expected_params) = &self.expected_params {
            assert_eq!(&params, expected_params, "Parameters mismatch");
        }

        Ok(Box::new(MockQueryResult::new(
            self.column_names.clone(),
            self.return_data.clone(),
        )))
    }
}

pub struct MockQueryResultRow {
    pub values: Vec<String>,
}

impl QueryResultRow for MockQueryResultRow {
    fn get_string_value(&self, index: usize) -> Result<String, Error> {
        self.values
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Index {} out of bounds", index))
    }

    fn get_int_value(&self, index: usize) -> Result<i64, Error> {
        self.values
            .get(index)
            .and_then(|value| value.parse::<i64>().ok())
            .ok_or_else(|| anyhow!("Index {} out of bounds", index))
    }

    fn get_uint_value(&self, index: usize) -> Result<u64, Error> {
        self.values
            .get(index)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| anyhow!("Index {} out of bounds", index))
    }

    fn count(&self) -> usize {
        self.values.len()
    }
}

pub struct MockQueryResult {
    pub column_names: Vec<String>,
    pub rows: std::vec::IntoIter<Box<dyn QueryResultRow>>,
}

impl MockQueryResult {
    pub fn new(column_names: Vec<String>, data: Vec<Vec<String>>) -> Self {
        let rows: Vec<Box<dyn QueryResultRow>> = data
            .into_iter()
            .map(|values| Box::new(MockQueryResultRow { values }) as Box<dyn QueryResultRow>)
            .collect();

        Self {
            column_names,
            rows: rows.into_iter(),
        }
    }
}

impl QueryResult for MockQueryResult {
    fn get_column_names(&self) -> &Vec<String> {
        &self.column_names
    }

    fn next(&mut self) -> Option<Box<dyn QueryResultRow>> {
        self.rows.next()
    }
}
