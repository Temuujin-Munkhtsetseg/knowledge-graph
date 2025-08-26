use crate::kuzu::config::DatabaseConfig;
use kuzu::{Database, SystemConfig};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

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

    pub fn drop_database(&self, database_path: &str) {
        let mut databases_guard = self.databases.lock().unwrap();
        databases_guard.remove(database_path);
    }

    pub fn get_or_create_database(
        &self,
        database_path: &str,
        config: Option<DatabaseConfig>,
    ) -> Option<Arc<Database>> {
        let mut databases_guard = self.databases.lock().unwrap();

        if databases_guard.contains_key(database_path) {
            info!(
                "KuzuDatabase::get_or_create_database -Found existing arc(database): {:?}",
                databases_guard.get(database_path).unwrap()
            );
            return Some(databases_guard.get(database_path).unwrap().clone());
        }

        let already_exists = std::path::Path::new(database_path).exists();
        info!(
            "KuzuDatabase::get_or_create_database - Database already exists in filesystem: {already_exists}"
        );

        let database = if let Some(config) = config {
            let system_config = config.fmt_kuzu_database_config();
            Database::new(database_path, system_config)
        } else {
            Database::new(database_path, SystemConfig::default())
        };

        if database.is_err() {
            error!(
                "KuzuDatabase::get_or_create_database - Failed to create database error: {:?}",
                database.err()
            );
            return None;
        } else {
            info!("KuzuDatabase::get_or_create_database - Database created at: {database_path}");
        }

        let database_arc = Arc::new(database.unwrap());
        databases_guard.insert(database_path.to_string(), database_arc.clone());
        Some(database_arc)
    }

    pub fn force_new_database(
        &self,
        database_path: &str,
        config: Option<DatabaseConfig>,
    ) -> Option<Arc<Database>> {
        // optionally remove the database from the map and the file system, called during a fresh indexing job
        if std::path::Path::new(database_path).exists() {
            self.drop_database(database_path);
            match std::fs::metadata(database_path) {
                Ok(metadata) => {
                    let removal_result = if metadata.is_dir() {
                        std::fs::remove_dir_all(database_path)
                    } else {
                        std::fs::remove_file(database_path)
                    };

                    if let Err(e) = removal_result {
                        error!(
                            "KuzuDatabase::force_new_database - Failed to remove existing database path '{}': {}",
                            database_path, e
                        );
                        return None;
                    }

                    info!(
                        "KuzuDatabase::force_new_database - Force reset database at: {database_path}"
                    );
                }
                Err(e) => {
                    error!(
                        "KuzuDatabase::force_new_database - Failed to stat existing database path '{}': {}",
                        database_path, e
                    );
                    return None;
                }
            }
        } else {
            info!("KuzuDatabase::force_new_database - Database not found at: {database_path}");
        }
        self.get_or_create_database(database_path, config)
    }
}
