use crate::kuzu::{
    database::KuzuQueryResult,
    types::{DatabaseError, DatabaseStats},
};

use anyhow::Error;
use kuzu::{Connection, Database};
use serde_json::Map;
use tracing::{debug, error, info};

pub struct KuzuConnection<'a> {
    connection: Connection<'a>,
}

impl<'a> KuzuConnection<'a> {
    pub fn new(database: &'a Database) -> Result<Self, Error> {
        let connection = Connection::new(database);

        if connection.is_err() {
            return Err(Error::msg(
                "Failed to create connection to database.".to_string(),
            ));
        }

        Ok(Self {
            connection: connection.unwrap(),
        })
    }

    pub fn generic_query(
        &self,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<KuzuQueryResult, Error> {
        let kuzu_params = extract_kuzu_params(&params);
        let mut prepared = self.connection.prepare(query)?;

        let result = self.connection.execute(&mut prepared, kuzu_params)?;

        Ok(KuzuQueryResult {
            column_names: result.get_column_names().to_vec(),
            result: result.into_iter().collect::<Vec<_>>(),
        })
    }

    pub fn query(&self, query: &str) -> Result<kuzu::QueryResult<'_>, DatabaseError> {
        self.connection
            .query(query)
            .map_err(|e| DatabaseError::QueryExecutionError {
                query: query.to_string(),
                error: e,
            })
    }

    /// Execute a prepared statement with parameters
    pub fn execute(
        &self,
        statement: &mut kuzu::PreparedStatement,
        params: Vec<(&str, kuzu::Value)>,
    ) -> Result<kuzu::QueryResult<'_>, DatabaseError> {
        debug!(
            "Executing prepared statement with {} parameters",
            params.len()
        );

        self.connection
            .execute(statement, params)
            .map_err(DatabaseError::Kuzu)
    }

    pub fn execute_ddl(&self, query: &str) -> Result<(), DatabaseError> {
        debug!("Executing DDL: {}", query);

        let mut prepared = self.connection.prepare(query)?;
        let mut result = self.connection.execute(&mut prepared, vec![])?;

        // Consume the result to ensure the query executed
        while result.next().is_some() {
            // DDL queries typically don't return data, but we consume any results
        }

        Ok(())
    }

    fn start_transaction(&self) -> Result<(), DatabaseError> {
        let mut prepared = self
            .connection
            .prepare("BEGIN TRANSACTION;")
            .expect("Failed to prepare begin transaction");
        self.connection
            .execute(&mut prepared, vec![])
            .expect("Failed to begin transaction");
        Ok(())
    }

    fn commit_transaction(&self) -> Result<(), DatabaseError> {
        let mut prepared = self
            .connection
            .prepare("COMMIT;")
            .expect("Failed to prepare commit transaction");
        self.connection
            .execute(&mut prepared, vec![])
            .expect("Failed to commit transaction");
        Ok(())
    }

    pub fn transaction(
        &mut self,
        f: impl FnOnce(&mut KuzuConnection) -> Result<(), DatabaseError>,
    ) -> Result<(), DatabaseError> {
        self.start_transaction()?;
        f(self)?;
        self.commit_transaction()?;
        Ok(())
    }

    pub fn copy_nodes_from_parquet(
        &self,
        table_name: &str,
        file_path: &str,
    ) -> Result<(), DatabaseError> {
        let absolute_path = std::path::Path::new(file_path)
            .canonicalize()
            .map_err(DatabaseError::Io)?;

        // For Parquet files, we don't need HEADER option as schema is embedded
        // Schema information is stored in Parquet metadata
        let query = format!("COPY {} FROM '{}'", table_name, absolute_path.display());

        info!("Importing data into {}: {}", table_name, file_path);
        self.execute_ddl(&query).map_err(|e| {
            error!(
                "Failed to import data into table {} from file {}: {}",
                table_name, file_path, e
            );
            e
        })?;
        info!("Successfully imported data into {}", table_name);

        Ok(())
    }

    /// Bulk import relationships from a Parquet file with specific FROM/TO types
    pub fn copy_relationships_from_parquet(
        &self,
        table_name: &str,
        file_path: &str,
        from_table: Option<&str>,
        to_table: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let absolute_path = std::path::Path::new(file_path)
            .canonicalize()
            .map_err(DatabaseError::Io)?;

        let mut query = format!("COPY {} FROM '{}'", table_name, absolute_path.display());

        // For Parquet files, only from/to options are needed for relationship tables
        // HEADER is not needed as schema is embedded in Parquet metadata
        let mut options = Vec::new();
        if let Some(from) = from_table {
            options.push(format!("from='{from}'"));
        }

        if let Some(to) = to_table {
            options.push(format!("to='{to}'"));
        }

        if !options.is_empty() {
            let options_str: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
            query.push_str(&format!(" ({})", options_str.join(", ")));
        }

        info!(
            "Importing relationship data into {}: {}",
            table_name, file_path
        );
        self.execute_ddl(&query).map_err(|e| {
            error!(
                "Failed to import relationship data into table {} from file {}: {}",
                table_name, file_path, e
            );
            e
        })?;
        info!(
            "Successfully imported relationship data into {}",
            table_name
        );

        Ok(())
    }

    pub fn table_exists(&self, table_name: &str) -> Result<bool, DatabaseError> {
        let query = "CALL SHOW_TABLES() RETURN *";
        let result = self.connection.query(query)?;

        for row in result {
            if let Some(kuzu::Value::String(existing_table_name)) = row.get(1) {
                // Index 1 contains the table name
                if existing_table_name.eq_ignore_ascii_case(table_name) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn get_table_names(&self) -> Result<Vec<String>, DatabaseError> {
        let query = "CALL SHOW_TABLES() RETURN *";
        let result = self.query(query)?;
        let mut table_names = Vec::new();

        for row in result {
            if let Some(kuzu::Value::String(table_name)) = row.get(1) {
                // Index 1 contains the table name
                table_names.push(table_name.to_string());
            }
        }

        Ok(table_names)
    }

    pub fn get_database_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let table_names = self.get_table_names()?;
        let mut node_tables = 0;
        let mut rel_tables = 0;
        let mut total_nodes = 0;
        let mut total_relationships = 0;

        for table_name in &table_names {
            // Check if it's a node or relationship table by trying to count
            let count_query = format!("MATCH (n:{table_name}) RETURN count(n)");
            if let Ok(mut result) = self.query(&count_query) {
                if let Some(row) = result.next()
                    && let Some(kuzu::Value::Int64(count)) = row.first()
                {
                    total_nodes += count;
                    node_tables += 1;
                }
            } else {
                // Try as relationship table
                let rel_count_query = format!("MATCH ()-[r:{table_name}]-() RETURN count(r)");
                if let Ok(mut result) = self.query(&rel_count_query)
                    && let Some(row) = result.next()
                    && let Some(kuzu::Value::Int64(count)) = row.first()
                {
                    total_relationships += count;
                    rel_tables += 1;
                }
            }
        }

        Ok(DatabaseStats {
            total_tables: table_names.len(),
            node_tables,
            rel_tables,
            total_nodes: total_nodes as usize,
            total_relationships: total_relationships as usize,
        })
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
    mod generic_query_test {
        use crate::kuzu::{connection::KuzuConnection, database::KuzuDatabase};

        #[test]
        fn test_kuzu_query_with_no_params() {
            let temp_dir = tempfile::tempdir().unwrap();
            let binding = temp_dir.path().join("test.db");
            let database_path = binding.to_str().unwrap();
            let database = KuzuDatabase::new()
                .force_new_database(database_path, None)
                .unwrap();
            let conection = KuzuConnection::new(&database).unwrap();

            // create a simple table called Person with name and age
            conection
                .execute_ddl(
                    "CREATE NODE TABLE User (name STRING, age INT64 DEFAULT 0, PRIMARY KEY (name))",
                )
                .unwrap();
            conection
                .execute_ddl("CREATE (u:User {name: 'Alice', age: 35});")
                .unwrap();
            conection
                .execute_ddl("CREATE (u:User {name: 'Jane', age: 25});")
                .unwrap();

            let result = conection
                .generic_query(
                    "MATCH (n:User) RETURN n.name, n.age",
                    serde_json::Map::new(),
                )
                .unwrap();

            assert_eq!(result.result[0][0].to_string(), "Alice");
            assert_eq!(result.result[0][1].to_string(), "35");

            assert_eq!(result.result[1][0].to_string(), "Jane");
            assert_eq!(result.result[1][1].to_string(), "25");

            std::fs::remove_dir_all(temp_dir).unwrap();
        }

        #[test]
        fn test_kuzu_query_with_params() {
            let temp_dir = tempfile::tempdir().unwrap();
            let binding = temp_dir.path().join("test.db");
            let database_path = binding.to_str().unwrap();
            let database = KuzuDatabase::new()
                .force_new_database(database_path, None)
                .unwrap();
            let conection = KuzuConnection::new(&database).unwrap();

            // create a simple table called Person with id, name, age and vip column
            conection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age INT64 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
                .unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 2, name: 'Alice', age: 20, vip: true});")
                .unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 3, name: 'Jane', age: 25, vip: false});")
                .unwrap();

            let result = conection.generic_query(
                "MATCH (u:User) WHERE u.name = $name AND u.age = $age AND u.vip = $vip RETURN u.name, u.age",
                serde_json::json!({ "name": "Alice", "age": 20, "vip": true }).as_object().unwrap().clone()
            )
            .unwrap();

            assert_eq!(result.column_names[0], "u.name");
            assert_eq!(result.column_names[1], "u.age");

            assert_eq!(result.result[0][0].to_string(), "Alice");
            assert_eq!(result.result[0][1].to_string(), "20");

            std::fs::remove_dir_all(temp_dir).unwrap();
        }

        #[test]
        fn test_kuzu_query_with_string_list_params() {
            let temp_dir = tempfile::tempdir().unwrap();
            let binding = temp_dir.path().join("test.db");
            let database_path = binding.to_str().unwrap();
            let database = KuzuDatabase::new()
                .force_new_database(database_path, None)
                .unwrap();
            let conection = KuzuConnection::new(&database).unwrap();

            // create a simple table called User with id, name, age and vip column
            conection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age INT64 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
                .unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 2, name: 'Bob', age: 20, vip: true});")
                .unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 3, name: 'Charlie', age: 25, vip: false});")
                .unwrap();

            let result = conection
                .generic_query(
                    "MATCH (u:User) WHERE u.name IN $names RETURN u.name, u.age",
                    serde_json::json!({ "names": ["Alice", "Charlie"] })
                        .as_object()
                        .unwrap()
                        .clone(),
                )
                .unwrap();

            assert_eq!(result.result[0][0].to_string(), "Alice");
            assert_eq!(result.result[0][1].to_string(), "35");

            assert_eq!(result.result[1][0].to_string(), "Charlie");
            assert_eq!(result.result[1][1].to_string(), "25");

            std::fs::remove_dir_all(temp_dir).unwrap();
        }

        #[test]
        fn test_kuzu_query_with_int_list_params() {
            let temp_dir = tempfile::tempdir().unwrap();
            let binding = temp_dir.path().join("test.db");
            let database_path = binding.to_str().unwrap();
            let database: std::sync::Arc<kuzu::Database> = KuzuDatabase::new()
                .force_new_database(database_path, None)
                .unwrap();
            let conection = KuzuConnection::new(&database).unwrap();

            // create a simple table called User with id, name, age and vip column
            conection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age UINT8 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
                .unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 2, name: 'Bob', age: 20, vip: true});")
                .unwrap();

            let result = conection
                .generic_query(
                    "MATCH (u:User) WHERE u.id IN $ids RETURN u.name, u.age",
                    serde_json::json!({ "ids": [1, 2] })
                        .as_object()
                        .unwrap()
                        .clone(),
                )
                .unwrap();

            assert_eq!(result.result[0][0].to_string(), "Alice");
            assert_eq!(result.result[0][1].to_string(), "35");

            assert_eq!(result.result[1][0].to_string(), "Bob");
            assert_eq!(result.result[1][1].to_string(), "20");

            std::fs::remove_dir_all(temp_dir).unwrap();
        }

        #[test]
        fn test_kuzu_query_finds_no_results() {
            let temp_dir = tempfile::tempdir().unwrap();
            let binding = temp_dir.path().join("test.db");
            let database_path = binding.to_str().unwrap();
            let database = KuzuDatabase::new()
                .force_new_database(database_path, None)
                .unwrap();
            let conection = KuzuConnection::new(&database).unwrap();

            // create a simple table called User with id, name, age and vip column
            conection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age INT64 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
                .unwrap();

            let result = conection.generic_query(
                "MATCH (u:User) WHERE u.name = $name AND u.age = $age AND u.vip = $vip RETURN u.name, u.age",
                serde_json::json!({ "name": "Alice", "age": 20, "vip": true }).as_object().unwrap().clone(),
            )
            .unwrap();

            assert!(result.result.is_empty());

            std::fs::remove_dir_all(temp_dir).unwrap();
        }

        #[test]
        fn test_kuzu_query_with_invalid_params() {
            let temp_dir = tempfile::tempdir().unwrap();
            let binding = temp_dir.path().join("test.db");
            let database_path = binding.to_str().unwrap();
            let database = KuzuDatabase::new()
                .force_new_database(database_path, None)
                .unwrap();
            let conection = KuzuConnection::new(&database).unwrap();

            // create a simple table called User with id, name, age and vip column
            conection.execute_ddl("CREATE NODE TABLE User (id INT64, name STRING, age INT64 DEFAULT 0, vip BOOLEAN DEFAULT FALSE, PRIMARY KEY (id))").unwrap();
            conection
                .execute_ddl("CREATE (u:User {id: 1, name: 'Alice', age: 35, vip: true});")
                .unwrap();

            let result = conection.generic_query(
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

            let database = KuzuDatabase::new()
                .force_new_database(database_path, None)
                .unwrap();
            let conection = KuzuConnection::new(&database).unwrap();

            let result = conection.generic_query(
            "MATCH (u:User) WHERE u.name = $name AND u.age = $age AND u.vip = $vip RETURN u.name, u.age",
            serde_json::json!({ "name": "Alice", "age": 20, "vip": true }).as_object().unwrap().clone()
        );

            assert!(result.is_err());

            std::fs::remove_dir_all(temp_dir).unwrap();
        }
    }
}
