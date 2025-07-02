use anyhow::{Error, Result};
use serde_json::Map;

pub trait DatabaseConnection: Send + Sync {
    fn query(
        &self,
        database_path: &str,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<Box<dyn DatabaseResult>, Error>;
}

pub trait DatabaseResult: Send + Sync {
    fn get_column_names(&self) -> &Vec<String>;
    fn next(&mut self) -> Option<Box<dyn DatabaseRow>>;
}

impl dyn DatabaseResult {
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

pub trait DatabaseRow: Send + Sync {
    fn get_string_value(&self, index: usize) -> Result<String, Error>;
    fn count(&self) -> usize;
}

impl dyn DatabaseRow {
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use serde_json::json;

    struct MockDatabaseRow {
        values: Vec<String>,
    }

    impl DatabaseRow for MockDatabaseRow {
        fn get_string_value(&self, index: usize) -> Result<String, Error> {
            self.values
                .get(index)
                .cloned()
                .ok_or_else(|| anyhow!("index out of bounds"))
        }

        fn count(&self) -> usize {
            self.values.len()
        }
    }

    struct MockDatabaseResult {
        column_names: Vec<String>,
        rows: std::vec::IntoIter<Box<dyn DatabaseRow>>,
    }

    impl DatabaseResult for MockDatabaseResult {
        fn get_column_names(&self) -> &Vec<String> {
            &self.column_names
        }

        fn next(&mut self) -> Option<Box<dyn DatabaseRow>> {
            self.rows.next()
        }
    }

    #[test]
    fn test_database_row_to_json() {
        let row = MockDatabaseRow {
            values: vec!["value1".to_string(), "value2".to_string()],
        };
        let column_names = vec!["col1".to_string(), "col2".to_string()];
        let json_val = (&row as &dyn DatabaseRow).to_json(&column_names).unwrap();

        assert_eq!(json_val, json!({ "col1": "value1", "col2": "value2" }));
    }

    #[test]
    fn test_database_row_to_json_with_extra_values() {
        let row = MockDatabaseRow {
            values: vec![
                "value1".to_string(),
                "value2".to_string(),
                "value3".to_string(),
            ],
        };
        let column_names = vec!["col1".to_string(), "col2".to_string()];
        let json_val = (&row as &dyn DatabaseRow).to_json(&column_names).unwrap();

        assert_eq!(json_val, json!({ "col1": "value1", "col2": "value2" }));
    }

    #[test]
    fn test_database_row_to_json_with_mismatched_columns() {
        let row = MockDatabaseRow {
            values: vec!["value1".to_string()],
        };
        let column_names = vec!["col1".to_string(), "col2".to_string()];
        let json_val = (&row as &dyn DatabaseRow).to_json(&column_names).unwrap();

        assert_eq!(json_val, json!({ "col1": "value1" }));
    }

    #[test]
    fn test_database_result_to_json() {
        let row1 = MockDatabaseRow {
            values: vec!["r1c1".to_string(), "r1c2".to_string()],
        };
        let row2 = MockDatabaseRow {
            values: vec!["r2c1".to_string(), "r2c2".to_string()],
        };

        let mut result = MockDatabaseResult {
            column_names: vec!["col1".to_string(), "col2".to_string()],
            rows: vec![
                Box::new(row1) as Box<dyn DatabaseRow>,
                Box::new(row2) as Box<dyn DatabaseRow>,
            ]
            .into_iter(),
        };

        let json_val = (&mut result as &mut dyn DatabaseResult).to_json().unwrap();

        assert_eq!(
            json_val,
            json!([
                { "col1": "r1c1", "col2": "r1c2" },
                { "col1": "r2c1", "col2": "r2c2" }
            ])
        );
    }

    #[test]
    fn test_database_result_to_json_with_empty_result() {
        let mut result = MockDatabaseResult {
            column_names: vec!["col1".to_string(), "col2".to_string()],
            rows: vec![].into_iter(),
        };

        let json_val = (&mut result as &mut dyn DatabaseResult).to_json().unwrap();

        assert_eq!(json_val, json!([]));
    }
}
