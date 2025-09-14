use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::{collections::HashSet, sync::Arc};

use log::{info, warn};
use serde::{Deserialize, Serialize};
use workspace_manager::WorkspaceManager;

const MCP_CONFIGURATION_FILE_NAME: &str = "mcp.settings.json";

#[derive(Serialize, Deserialize)]
pub struct McpConfiguration {
    pub disabled_tools: HashSet<String>,
}

impl McpConfiguration {
    pub fn new() -> Self {
        Self {
            disabled_tools: HashSet::new(),
        }
    }

    pub fn is_tool_enabled(&self, tool_name: &str) -> bool {
        !self.disabled_tools.contains(tool_name)
    }

    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

impl Default for McpConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

pub fn read_mcp_configuration(path: PathBuf) -> McpConfiguration {
    let content = fs::read_to_string(path);

    if let Err(e) = content {
        warn!(
            "Could not parse MCP configuration: {}. Returning default configuration.",
            e
        );
        return McpConfiguration::default();
    }

    match serde_json::from_str(&content.unwrap()) {
        Ok(configuration) => configuration,
        Err(e) => {
            warn!(
                "Could not parse MCP configuration: {}. Returning default configuration.",
                e
            );
            McpConfiguration::default()
        }
    }
}

pub fn get_or_create_mcp_configuration(
    workspace_folder_path: Arc<WorkspaceManager>,
) -> McpConfiguration {
    // Get data directory info, return default if unavailable
    let data_directory_info = match workspace_folder_path.get_data_directory_info() {
        Ok(info) if info.root_path.exists() => info,
        Ok(_) => {
            warn!("Data directory does not exist, returning default MCP configuration.");
            return McpConfiguration::default();
        }
        Err(e) => {
            warn!(
                "Could not get data directory info: {}. Returning default MCP configuration.",
                e
            );
            return McpConfiguration::default();
        }
    };

    let configuration_path = data_directory_info
        .root_path
        .join(MCP_CONFIGURATION_FILE_NAME);

    // If config file doesn't exist, create it
    if !configuration_path.exists() {
        let new_configuration = McpConfiguration::default();
        if let Err(e) = new_configuration.save(&configuration_path) {
            warn!(
                "Could not save MCP configuration: {}. Returning default configuration.",
                e
            );
            return McpConfiguration::default();
        }

        info!(
            "Created new MCP configuration file at {}.",
            configuration_path.display()
        );
        return new_configuration;
    }

    // Read and parse existing config file
    match fs::read_to_string(&configuration_path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(configuration) => configuration,
            Err(e) => {
                warn!(
                    "Could not parse MCP configuration: {}. Returning default configuration.",
                    e
                );
                McpConfiguration::default()
            }
        },
        Err(e) => {
            warn!(
                "Could not read MCP configuration: {}. Returning default configuration.",
                e
            );
            McpConfiguration::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use workspace_manager::{DataDirectory, LocalStateService};

    const FRAMEWORK_VERSION: &str = "0.12.0";

    fn create_test_workspace_manager() -> (Arc<WorkspaceManager>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let data_directory = DataDirectory::new(temp_dir.path().to_path_buf()).unwrap();
        let state_service =
            LocalStateService::new(&data_directory.manifest_path, FRAMEWORK_VERSION.to_string())
                .unwrap();
        let manager = Arc::new(WorkspaceManager::new(data_directory, state_service));
        (manager, temp_dir)
    }

    #[test]
    fn test_default_configuration() {
        let config = McpConfiguration::default();
        assert!(config.disabled_tools.is_empty());

        let new_config = McpConfiguration::new();
        assert_eq!(config.disabled_tools.len(), new_config.disabled_tools.len());
    }

    #[test]
    fn test_configuration_save_success() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        let mut config = McpConfiguration::new();
        config.disabled_tools.insert("test_tool".to_string());

        let result = config.save(&config_path);
        assert!(result.is_ok());
        assert!(config_path.exists());

        // Verify the file contents
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("disabled_tools"));
        assert!(content.contains("test_tool"));
    }

    #[test]
    fn test_configuration_save_invalid_path() {
        let config = McpConfiguration::new();
        let invalid_path = PathBuf::from("/nonexistent/directory/config.json");

        let result = config.save(&invalid_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_or_create_mcp_configuration_creates_new_file() {
        let (manager, _temp_dir) = create_test_workspace_manager();

        let config = get_or_create_mcp_configuration(manager.clone());
        assert!(config.disabled_tools.is_empty());

        // Verify the config file was created
        let data_info = manager.get_data_directory_info().unwrap();
        let config_path = data_info.root_path.join(MCP_CONFIGURATION_FILE_NAME);
        assert!(config_path.exists());
    }

    #[test]
    fn test_get_or_create_mcp_configuration_loads_existing_file() {
        let (manager, _temp_dir) = create_test_workspace_manager();

        // First call creates the file
        let _config1 = get_or_create_mcp_configuration(manager.clone());

        // Modify the config file manually to test loading
        let data_info = manager.get_data_directory_info().unwrap();
        let config_path = data_info.root_path.join(MCP_CONFIGURATION_FILE_NAME);

        let mut modified_config = McpConfiguration::new();
        modified_config
            .disabled_tools
            .insert("test_disabled_tool".to_string());
        modified_config.save(&config_path).unwrap();

        // Second call should load the modified file
        let config2 = get_or_create_mcp_configuration(manager);
        assert_eq!(config2.disabled_tools.len(), 1);
        assert!(config2.disabled_tools.contains("test_disabled_tool"));
    }

    #[test]
    fn test_get_or_create_mcp_configuration_handles_corrupted_file() {
        let (manager, _temp_dir) = create_test_workspace_manager();

        // Create a corrupted config file
        let data_info = manager.get_data_directory_info().unwrap();
        let config_path = data_info.root_path.join(MCP_CONFIGURATION_FILE_NAME);
        fs::write(&config_path, "invalid json content").unwrap();

        // Should return default configuration when file is corrupted
        let config = get_or_create_mcp_configuration(manager);
        assert!(config.disabled_tools.is_empty());
    }

    #[test]
    fn test_get_or_create_mcp_configuration_handles_unreadable_file() {
        let (manager, _temp_dir) = create_test_workspace_manager();

        // Create a directory where the config file should be (making it unreadable as a file)
        let data_info = manager.get_data_directory_info().unwrap();
        let config_path = data_info.root_path.join(MCP_CONFIGURATION_FILE_NAME);
        fs::create_dir_all(&config_path).unwrap();

        // Should return default configuration when file cannot be read
        let config = get_or_create_mcp_configuration(manager);
        assert!(config.disabled_tools.is_empty());
    }

    #[allow(clippy::permissions_set_readonly_false)] //
    #[test]
    fn test_get_or_create_mcp_configuration_handles_save_failure() {
        let (manager, _temp_dir) = create_test_workspace_manager();

        // Make the data directory read-only to cause save failure
        let data_info = manager.get_data_directory_info().unwrap();
        let mut permissions = fs::metadata(&data_info.root_path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&data_info.root_path, permissions).unwrap();

        // Should still return default configuration even if save fails
        let config = get_or_create_mcp_configuration(manager);
        assert!(config.disabled_tools.is_empty());

        // Clean up: restore write permissions
        let mut permissions = fs::metadata(&data_info.root_path).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&data_info.root_path, permissions).unwrap();
    }

    #[test]
    fn test_configuration_serialization_and_deserialization() {
        let mut original_config = McpConfiguration::new();
        original_config.disabled_tools.insert("tool1".to_string());
        original_config.disabled_tools.insert("tool2".to_string());

        let json = serde_json::to_string_pretty(&original_config).unwrap();
        let deserialized_config: McpConfiguration = serde_json::from_str(&json).unwrap();

        assert_eq!(
            original_config.disabled_tools.len(),
            deserialized_config.disabled_tools.len()
        );
        assert!(deserialized_config.disabled_tools.contains("tool1"));
        assert!(deserialized_config.disabled_tools.contains("tool2"));
    }

    #[test]
    fn test_read_mcp_configuration_with_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("valid_config.json");

        // Create a valid configuration file
        let mut original_config = McpConfiguration::new();
        original_config
            .disabled_tools
            .insert("test_tool_1".to_string());
        original_config
            .disabled_tools
            .insert("test_tool_2".to_string());
        original_config.save(&config_path).unwrap();

        // Read the configuration
        let loaded_config = read_mcp_configuration(config_path);

        assert_eq!(loaded_config.disabled_tools.len(), 2);
        assert!(loaded_config.disabled_tools.contains("test_tool_1"));
        assert!(loaded_config.disabled_tools.contains("test_tool_2"));
    }

    #[test]
    fn test_read_mcp_configuration_with_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid_config.json");

        // Write invalid JSON content
        fs::write(&config_path, "{ invalid json content }").unwrap();

        // Should return default configuration
        let config = read_mcp_configuration(config_path);
        assert!(config.disabled_tools.is_empty());
    }

    #[test]
    fn test_read_mcp_configuration_with_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent_config.json");

        // Should return default configuration when file doesn't exist
        let config = read_mcp_configuration(config_path);
        assert!(config.disabled_tools.is_empty());
    }

    #[test]
    fn test_read_mcp_configuration_with_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("empty_config.json");

        // Create an empty file
        fs::write(&config_path, "").unwrap();

        // Should return default configuration
        let config = read_mcp_configuration(config_path);
        assert!(config.disabled_tools.is_empty());
    }

    #[test]
    fn test_read_mcp_configuration_with_malformed_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("malformed_config.json");

        // Write valid JSON but wrong structure
        fs::write(&config_path, r#"{"wrong_field": ["tool1", "tool2"]}"#).unwrap();

        // Should return default configuration with empty disabled_tools
        let config = read_mcp_configuration(config_path);
        assert!(config.disabled_tools.is_empty());
    }

    #[test]
    fn test_read_mcp_configuration_with_partial_structure() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("partial_config.json");

        // Write JSON with missing disabled_tools field (should use default)
        fs::write(&config_path, r#"{}"#).unwrap();

        // Should return configuration with empty disabled_tools (serde default)
        let config = read_mcp_configuration(config_path);
        assert!(config.disabled_tools.is_empty());
    }

    #[test]
    fn test_read_mcp_configuration_with_directory_instead_of_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config_as_directory");

        // Create a directory where file should be
        fs::create_dir_all(&config_path).unwrap();

        // Should return default configuration when trying to read directory as file
        let config = read_mcp_configuration(config_path);
        assert!(config.disabled_tools.is_empty());
    }
}
