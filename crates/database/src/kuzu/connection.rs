use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::kuzu::result::KuzuDatabaseResult;
use crate::{DatabaseConnection, DatabaseResult};
use anyhow::{Error, Result};
use kuzu::{Connection, Database, SystemConfig};
use serde_json::Map;

pub struct KuzuDatabaseConnection {
    databases: Mutex<HashMap<String, Arc<Database>>>,
}

impl Default for KuzuDatabaseConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl KuzuDatabaseConnection {
    pub fn new() -> Self {
        Self {
            databases: Mutex::new(HashMap::new()),
        }
    }
}

impl DatabaseConnection for KuzuDatabaseConnection {
    fn query(
        &self,
        database_path: &str,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<Box<dyn DatabaseResult>, Error> {
        let database = self.get_or_create_database(database_path);

        if database.is_none() {
            return Err(Error::msg(format!("Database not found: {database_path}.")));
        }

        let database = database.unwrap();
        let connection = Connection::new(&database);

        if connection.is_err() {
            return Err(Error::msg(format!(
                "Failed to create connection to database: {database_path}."
            )));
        }

        let connection = connection.unwrap();
        let kuzu_params = extract_kuzu_params(&params);
        let mut prepared = connection.prepare(query)?;
        let result = connection.execute(&mut prepared, kuzu_params)?;

        Ok(Box::new(KuzuDatabaseResult::new(
            result.get_column_names(),
            result.into_iter().collect::<Vec<_>>(),
        )))
    }
}

impl KuzuDatabaseConnection {
    pub fn get_or_create_database(&self, database_path: &str) -> Option<Arc<Database>> {
        let mut databases_guard = self.databases.lock().unwrap();

        if databases_guard.contains_key(database_path) {
            return Some(databases_guard.get(database_path).unwrap().clone());
        }

        let database = Database::new(database_path, SystemConfig::default());
        if database.is_err() {
            return None;
        }

        let database_arc = Arc::new(database.unwrap());
        databases_guard.insert(database_path.to_string(), database_arc.clone());
        Some(database_arc)
    }
}

fn extract_kuzu_params(
    json_params: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(&str, kuzu::Value)> {
    json_params
        .iter()
        .map(|(key, value)| (key.as_str(), convert_json_to_kuzu_value(value)))
        .collect()
}

fn convert_json_to_kuzu_value(value: &serde_json::Value) -> kuzu::Value {
    match value {
        serde_json::Value::String(s) => kuzu::Value::from(s.as_str()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                kuzu::Value::from(i)
            } else if let Some(f) = n.as_f64() {
                kuzu::Value::from(f)
            } else {
                kuzu::Value::from(0i64)
            }
        }
        serde_json::Value::Bool(b) => kuzu::Value::Bool(*b),
        serde_json::Value::Null => kuzu::Value::Null(kuzu::LogicalType::Any),
        serde_json::Value::Array(arr) => {
            let mut values = Vec::with_capacity(arr.len());

            for item in arr {
                values.push(convert_json_to_kuzu_value(item));
            }

            let logical_type = if let Some(first_item) = arr.first() {
                match first_item {
                    serde_json::Value::String(_) => kuzu::LogicalType::String,
                    serde_json::Value::Number(n) => {
                        if n.is_i64() {
                            kuzu::LogicalType::Int64
                        } else {
                            kuzu::LogicalType::Double
                        }
                    }
                    serde_json::Value::Bool(_) => kuzu::LogicalType::Bool,
                    _ => kuzu::LogicalType::Any,
                }
            } else {
                kuzu::LogicalType::Any
            };

            kuzu::Value::List(logical_type, values)
        }
        serde_json::Value::Object(obj) => {
            let mut map_values = Vec::with_capacity(obj.len());
            for (key, val) in obj {
                let converted_value = convert_json_to_kuzu_value(val);
                map_values.push((key.to_string(), converted_value));
            }
            kuzu::Value::Struct(map_values)
        }
    }
}

#[cfg(test)]
mod tests {
    use indexer::{DatabaseConfig, KuzuConnection};

    use super::*;

    #[test]
    fn test_kuzu_query_with_no_params() {
        let temp_dir = tempfile::tempdir().unwrap();
        let binding = temp_dir.path().join("test.db");
        let database_path = binding.to_str().unwrap();
        let config = DatabaseConfig::new(database_path);
        let database = KuzuConnection::create_database(config).unwrap();
        let base_connection = KuzuConnection::new(&database, database_path.to_string()).unwrap();

        // create a simple table called Person with name and age
        base_connection
            .execute_ddl(
                "CREATE NODE TABLE User (name STRING, age INT64 DEFAULT 0, PRIMARY KEY (name))",
            )
            .unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {name: 'Alice', age: 35});")
            .unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {name: 'Jane', age: 25});")
            .unwrap();

        let connection = KuzuDatabaseConnection::new();

        let mut result = connection
            .query(
                database_path,
                "MATCH (n:User) RETURN n.name, n.age",
                serde_json::Map::new(),
            )
            .unwrap();

        let result_iter = result.next().unwrap();
        assert_eq!(result_iter.get_string_value(0).unwrap(), "Alice");
        assert_eq!(result_iter.get_string_value(1).unwrap(), "35");

        let result_iter = result.next().unwrap();
        assert_eq!(result_iter.get_string_value(0).unwrap(), "Jane");
        assert_eq!(result_iter.get_string_value(1).unwrap(), "25");

        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_kuzu_query_with_params() {
        let temp_dir = tempfile::tempdir().unwrap();
        let binding = temp_dir.path().join("test.db");
        let database_path = binding.to_str().unwrap();
        let config = DatabaseConfig::new(database_path);
        let database = KuzuConnection::create_database(config).unwrap();
        let base_connection = KuzuConnection::new(&database, database_path.to_string()).unwrap();

        // create a simple table called Person with id, name, age and vip column
        base_connection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age INT64 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
            .unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 2, name: 'Alice', age: 20, vip: true});")
            .unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 3, name: 'Jane', age: 25, vip: false});")
            .unwrap();

        let connection = KuzuDatabaseConnection::new();

        let mut result = connection.query(
            database_path,
            "MATCH (u:User) WHERE u.name = $name AND u.age = $age AND u.vip = $vip RETURN u.name, u.age", 
            serde_json::json!({ "name": "Alice", "age": 20, "vip": true }).as_object().unwrap().clone()
        ).unwrap();

        let result_iter = result.next().unwrap();

        assert_eq!(result_iter.get_string_value(0).unwrap(), "Alice");
        assert_eq!(result_iter.get_string_value(1).unwrap(), "20");

        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_kuzu_query_with_string_list_params() {
        let temp_dir = tempfile::tempdir().unwrap();
        let binding = temp_dir.path().join("test.db");
        let database_path = binding.to_str().unwrap();
        let config = DatabaseConfig::new(database_path);
        let database = KuzuConnection::create_database(config).unwrap();
        let base_connection = KuzuConnection::new(&database, database_path.to_string()).unwrap();

        // create a simple table called User with id, name, age and vip column
        base_connection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age INT64 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
            .unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 2, name: 'Bob', age: 20, vip: true});")
            .unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 3, name: 'Charlie', age: 25, vip: false});")
            .unwrap();

        let connection = KuzuDatabaseConnection::new();

        let mut result = connection
            .query(
                database_path,
                "MATCH (u:User) WHERE u.name IN $names RETURN u.name, u.age",
                serde_json::json!({ "names": ["Alice", "Charlie"] })
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .unwrap();

        let result_iter = result.next().unwrap();
        assert_eq!(result_iter.get_string_value(0).unwrap(), "Alice");
        assert_eq!(result_iter.get_string_value(1).unwrap(), "35");

        let result_iter = result.next().unwrap();
        assert_eq!(result_iter.get_string_value(0).unwrap(), "Charlie");
        assert_eq!(result_iter.get_string_value(1).unwrap(), "25");

        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_kuzu_query_with_int_list_params() {
        let temp_dir = tempfile::tempdir().unwrap();
        let binding = temp_dir.path().join("test.db");
        let database_path = binding.to_str().unwrap();
        let config = DatabaseConfig::new(database_path);
        let database = KuzuConnection::create_database(config).unwrap();
        let base_connection = KuzuConnection::new(&database, database_path.to_string()).unwrap();

        // create a simple table called User with id, name, age and vip column
        base_connection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age UINT8 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
            .unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 2, name: 'Bob', age: 20, vip: true});")
            .unwrap();

        let connection = KuzuDatabaseConnection::new();

        let mut result = connection
            .query(
                database_path,
                "MATCH (u:User) WHERE u.id IN $ids RETURN u.name, u.age",
                serde_json::json!({ "ids": [1, 2] })
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .unwrap();

        let result_iter = result.next().unwrap();
        assert_eq!(result_iter.get_string_value(0).unwrap(), "Alice");
        assert_eq!(result_iter.get_string_value(1).unwrap(), "35");

        let result_iter = result.next().unwrap();
        assert_eq!(result_iter.get_string_value(0).unwrap(), "Bob");
        assert_eq!(result_iter.get_string_value(1).unwrap(), "20");

        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_kuzu_query_finds_no_results() {
        let temp_dir = tempfile::tempdir().unwrap();
        let binding = temp_dir.path().join("test.db");
        let database_path = binding.to_str().unwrap();
        let config = DatabaseConfig::new(database_path);
        let database = KuzuConnection::create_database(config).unwrap();
        let base_connection = KuzuConnection::new(&database, database_path.to_string()).unwrap();

        // create a simple table called User with id, name, age and vip column
        base_connection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age INT64 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
            .unwrap();

        let connection = KuzuDatabaseConnection::new();

        let mut result = connection.query(
            database_path,
            "MATCH (u:User) WHERE u.name = $name AND u.age = $age AND u.vip = $vip RETURN u.name, u.age",
            serde_json::json!({ "name": "Alice", "age": 20, "vip": true }).as_object().unwrap().clone()
        ).unwrap();

        assert!(result.next().is_none());

        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_kuzu_query_with_invalid_params() {
        let temp_dir = tempfile::tempdir().unwrap();
        let binding = temp_dir.path().join("test.db");
        let database_path = binding.to_str().unwrap();
        let config = DatabaseConfig::new(database_path);
        let database = KuzuConnection::create_database(config).unwrap();
        let base_connection = KuzuConnection::new(&database, database_path.to_string()).unwrap();

        // create a simple table called User with id, name, age and vip column
        base_connection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age INT64 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
        base_connection
            .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
            .unwrap();

        let connection = KuzuDatabaseConnection::new();

        let result = connection.query(
            database_path,
            "MATCH (u:User) WHERE u.name = $name AND u.age = $age AND u.vip = $vip RETURN u.name, u.age",
            serde_json::json!({ "invalid": "invalid" }).as_object().unwrap().clone()
        );

        assert!(result.is_err());

        std::fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_kuzu_query_with_invalid_query() {
        let temp_dir = tempfile::tempdir().unwrap();
        let binding = temp_dir.path().join("test.db");
        let database_path = binding.to_str().unwrap();

        let connection = KuzuDatabaseConnection::new();

        let result = connection.query(
            database_path,
            "MATCH (u:User) WHERE u.name = $name AND u.age = $age AND u.vip = $vip RETURN u.name, u.age",
            serde_json::json!({ "name": "Alice", "age": 20, "vip": true }).as_object().unwrap().clone()
        );

        assert!(result.is_err());

        std::fs::remove_dir_all(temp_dir).unwrap();
    }
}
