use anyhow::Error;
use serde_json::Map;

pub trait QueryingService: Send + Sync {
    fn execute_query(
        &self,
        project_path: &str,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<Box<dyn QueryResult>, Error>;
}

pub trait QueryResult: Send + Sync {
    fn get_column_names(&self) -> &Vec<String>;
    fn next(&mut self) -> Option<Box<dyn QueryResultRow>>;
}

impl dyn QueryResult {
    pub fn to_json(&mut self) -> Result<serde_json::Value, Error> {
        let mut rows = Vec::new();

        while let Some(row) = self.next() {
            let json_row = row.to_json(self.get_column_names());

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
    fn count(&self) -> usize;
}

impl dyn QueryResultRow {
    pub fn to_json(&self, column_names: &[String]) -> Result<serde_json::Value, Error> {
        let mut json = serde_json::Map::with_capacity(column_names.len());

        for (i, column_name) in column_names.iter().enumerate().take(self.count()) {
            let value = self.get_string_value(i);

            if value.is_err() {
                continue;
            }

            json.insert(
                column_name.clone(),
                serde_json::Value::String(value.unwrap()),
            );
        }

        Ok(serde_json::Value::Object(json))
    }
}
