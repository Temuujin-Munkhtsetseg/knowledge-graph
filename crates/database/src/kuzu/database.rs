use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use kuzu::{Database, SystemConfig};

use tracing::info;

use crate::kuzu::config::DatabaseConfig;
use crate::kuzu::types::DatabaseError;

pub struct KuzuQueryResult {
    pub column_names: Vec<String>,
    pub result: Vec<Vec<kuzu::Value>>,
}

pub struct KuzuDatabase {
    databases: Mutex<HashMap<String, Arc<Database>>>,
}

impl Default for KuzuDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl KuzuDatabase {
    pub fn new() -> Self {
        Self {
            databases: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_database_keys(&self) -> Vec<String> {
        let databases_guard = self.databases.lock().unwrap();
        databases_guard.keys().cloned().collect()
    }

    pub fn get_or_create_database(&self, database_path: &str) -> Option<Arc<Database>> {
        let mut databases_guard = self.databases.lock().unwrap();

        if databases_guard.contains_key(database_path) {
            return Some(databases_guard.get(database_path).unwrap().clone());
        }

        let database = Database::new(database_path, SystemConfig::default());
        if database.is_err() {
            info!(
                "KuzuDatabase::get_or_create_database - Failed to create database error: {:?}",
                database.err()
            );
            return None;
        }

        let database_arc = Arc::new(database.unwrap());
        databases_guard.insert(database_path.to_string(), database_arc.clone());
        Some(database_arc)
    }

    pub fn create_temporary_database(
        &self,
        config: DatabaseConfig,
    ) -> Result<Database, DatabaseError> {
        info!(
            "Creating Kuzu database from config at: {}",
            config.database_path
        );

        // Delete existing database if it exists to start fresh
        let db_path = Path::new(&config.database_path);
        if db_path.exists() {
            info!("Removing existing database at: {}", config.database_path);
            if db_path.is_dir() {
                std::fs::remove_dir_all(db_path).map_err(|e| {
                    DatabaseError::InitializationFailed(format!(
                        "Failed to remove existing database directory: {e}"
                    ))
                })?;
            } else {
                std::fs::remove_file(db_path).map_err(|e| {
                    DatabaseError::InitializationFailed(format!(
                        "Failed to remove existing database file: {e}"
                    ))
                })?;
            }
        }

        // Create database directory if it doesn't exist
        if let Some(parent) = Path::new(&config.database_path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    DatabaseError::InitializationFailed(format!(
                        "Failed to create database directory: {e}"
                    ))
                })?;
            }
        }

        // Configure system settings
        let mut system_config = SystemConfig::default();

        if let Some(buffer_size) = config.buffer_pool_size {
            system_config = system_config.buffer_pool_size(buffer_size as u64);
        }

        if let Some(compression) = config.enable_compression {
            system_config = system_config.enable_compression(compression);
        }

        if let Some(read_only) = config.read_only {
            system_config = system_config.read_only(read_only);
        }

        if let Some(max_size) = config.max_db_size {
            system_config = system_config.max_db_size(max_size as u64);
        }

        // Create database
        let database = Database::new(&config.database_path, system_config)?;
        info!("Successfully created Kuzu database");

        Ok(database)
    }
}
