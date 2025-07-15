use kuzu::SystemConfig;

/// Database configuration options
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Path to the database file or directory
    pub database_path: String,
    /// Buffer pool size in bytes (default: 1GB)
    pub buffer_pool_size: Option<usize>,
    /// Whether to enable compression (default: true)
    pub enable_compression: Option<bool>,
    /// Whether to run in read-only mode (default: false)
    pub read_only: Option<bool>,
    /// Maximum database size in bytes (default: unlimited)
    pub max_db_size: Option<usize>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_path: "graph.db".to_string(),
            buffer_pool_size: Some(1024 * 1024 * 1024), // 1GB
            enable_compression: Some(true),
            read_only: Some(false),
            max_db_size: None,
        }
    }
}

impl DatabaseConfig {
    pub fn new<P: AsRef<str>>(database_path: P) -> Self {
        Self {
            database_path: database_path.as_ref().to_string(),
            ..Default::default()
        }
    }

    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_pool_size = Some(size);
        self
    }

    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.enable_compression = Some(enabled);
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = Some(true);
        self
    }

    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_db_size = Some(size);
        self
    }

    pub fn fmt_kuzu_database_config(&self) -> SystemConfig {
        // Configure system settings
        let mut system_config = SystemConfig::default();

        if let Some(buffer_size) = self.buffer_pool_size {
            system_config = system_config.buffer_pool_size(buffer_size as u64);
        }

        if let Some(compression) = self.enable_compression {
            system_config = system_config.enable_compression(compression);
        }

        if let Some(read_only) = self.read_only {
            system_config = system_config.read_only(read_only);
        }

        if let Some(max_size) = self.max_db_size {
            system_config = system_config.max_db_size(max_size as u64);
        }
        system_config
    }
}
