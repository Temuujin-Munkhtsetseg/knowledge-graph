use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::env::home_dir;
use std::fs;
use std::path::{MAIN_SEPARATOR, PathBuf};

use crate::MCP_NAME;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpServer {
    Url { url: String },
    Command { command: String, args: Vec<String> },
}

#[derive(Serialize, Deserialize)]

struct McpConfig {
    #[serde(skip)]
    path: PathBuf,

    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, McpServer>,
}

impl McpConfig {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            mcp_servers: HashMap::new(),
        }
    }

    pub fn get_or_create(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            fs::create_dir_all(path.parent().unwrap())?;
            return Ok(Self::new(path));
        }

        let content = fs::read_to_string(path.clone())?;
        let json: McpConfig = serde_json::from_str(&content)?;

        Ok(Self {
            path,
            mcp_servers: json.mcp_servers,
        })
    }

    pub fn add_server(mut self, name: String, server: McpServer) -> Self {
        self.mcp_servers.insert(name, server);

        self
    }

    pub fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(self.path.clone(), json)?;
        Ok(())
    }
}

// Helper function that adds the local HTTP server to the MCP configuration.
pub fn add_local_http_server_to_mcp_config(mcp_config_path: PathBuf, port: u16) -> Result<()> {
    let server = McpServer::Url {
        url: format!("http://localhost:{port}/mcp"),
    };

    let expanded_path = naively_expand_shell_path(mcp_config_path)?;

    McpConfig::get_or_create(expanded_path)?
        .add_server(MCP_NAME.to_string(), server)
        .save()
}

// Helper function that expands the shell variables in the path.
fn naively_expand_shell_path(path: PathBuf) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        expand_unix_path(path)
    }
    #[cfg(windows)]
    {
        expand_windows_path(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        // For other platforms, just return the path as-is
        Ok(path)
    }
}

// Unix-specific path expansion (handles tilde ~)
#[allow(unused)]
fn expand_unix_path(path: PathBuf) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();

    if let Some(without_tilde) = path_str.strip_prefix('~') {
        let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory."))?;
        if without_tilde.is_empty() || without_tilde.starts_with(MAIN_SEPARATOR) {
            let expanded_path = home.join(&without_tilde[1..]);
            return Ok(expanded_path);
        }
    }

    Ok(path)
}

// Windows-specific path expansion (handles %VAR%)
#[allow(unused)]
fn expand_windows_path(path: PathBuf) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    let mut expanded_path = path_str.to_string();

    // Handle Windows-style environment variables %VAR%
    if let Some(start) = expanded_path.find('%') {
        if let Some(end) = expanded_path[start + 1..].find('%') {
            let end = start + 1 + end;
            let var_name = &expanded_path[start + 1..end];

            match env::var(var_name) {
                Ok(var_value) => {
                    expanded_path.replace_range(start..=end, &var_value);
                }
                Err(_) => {
                    // If environment variable doesn't exist, leave it as is
                }
            }
        }
    }

    Ok(PathBuf::from(expanded_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_local_http_server_to_new_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");
        let port = 8080;

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), port).unwrap();

        let config = McpConfig::get_or_create(mcp_config_path.clone()).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);

        check_http_server_is_added_to_existing_config(
            config.mcp_servers.get(MCP_NAME).unwrap(),
            port,
        );

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_add_local_http_server_to_existing_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");
        let port = 8080;

        // Add a random server to the config
        McpConfig::get_or_create(mcp_config_path.clone())
            .unwrap()
            .add_server(
                "gitlab".to_string(),
                McpServer::Command {
                    command: "gitlab".to_string(),
                    args: vec!["mcp".to_string(), "server".to_string()],
                },
            )
            .save()
            .unwrap();

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), port).unwrap();

        let config = McpConfig::get_or_create(mcp_config_path.clone()).unwrap();
        assert_eq!(config.mcp_servers.len(), 2);

        check_http_server_is_added_to_existing_config(
            config.mcp_servers.get(MCP_NAME).unwrap(),
            port,
        );

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_overwrite_existing_server() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), 8080).unwrap();
        add_local_http_server_to_mcp_config(mcp_config_path.clone(), 8081).unwrap();

        let config = McpConfig::get_or_create(mcp_config_path.clone()).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);

        check_http_server_is_added_to_existing_config(
            config.mcp_servers.get(MCP_NAME).unwrap(),
            8081,
        );

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_add_local_http_server_to_non_existing_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("new-dir/mcp.json");
        let port = 8080;

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), port).unwrap();

        let config = McpConfig::get_or_create(mcp_config_path.clone()).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);

        check_http_server_is_added_to_existing_config(
            config.mcp_servers.get(MCP_NAME).unwrap(),
            port,
        );

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_expand_tilde_path() {
        let home_dir = home_dir().unwrap();
        let path = PathBuf::from("~/mcp.json");

        let expanded_path = expand_unix_path(path).unwrap();

        assert_eq!(expanded_path, home_dir.join("mcp.json"));
    }

    #[test]
    fn test_windows_expand_env_var_path() {
        // Set test environment variables
        unsafe {
            env::set_var("TEMP_APPDATA", "C:\\Users\\TestUser\\AppData\\Roaming");
        }

        let path = PathBuf::from("%TEMP_APPDATA%\\mcp.json");
        let expanded_path = expand_windows_path(path).unwrap();

        assert_eq!(
            expanded_path,
            PathBuf::from("C:\\Users\\TestUser\\AppData\\Roaming\\mcp.json")
        );

        // Clean up
        unsafe {
            env::remove_var("TEMP_APPDATA");
        }
    }

    #[test]
    fn test_windows_expand_nonexistent_env_var() {
        let path = PathBuf::from("%NONEXISTENT_VAR%\\mcp.json");
        let expanded_path = expand_windows_path(path).unwrap();

        // Should remain unchanged if environment variable doesn't exist
        assert_eq!(expanded_path, PathBuf::from("%NONEXISTENT_VAR%\\mcp.json"));
    }

    #[test]
    fn test_expand_regular_path() {
        // Test that regular paths without special characters work on all platforms
        let path = PathBuf::from("/some/regular/path/mcp.json");
        let expanded_path = naively_expand_shell_path(path.clone()).unwrap();

        assert_eq!(expanded_path, path);
    }

    fn check_http_server_is_added_to_existing_config(server: &McpServer, port: u16) {
        match server {
            McpServer::Url { url } => assert_eq!(*url, format!("http://localhost:{port}/mcp")),
            _ => panic!("Expected URL server"),
        }
    }
}
