use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::env::home_dir;
use std::fs;
use std::path::{MAIN_SEPARATOR, PathBuf};

use crate::MCP_NAME;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ApprovedTools {
    Bool(bool),
    Array(Vec<String>),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum DuoMcpServer {
    Url {
        #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
        command_type: Option<String>,
        url: String,
        #[serde(rename = "approvedTools", skip_serializing_if = "Option::is_none")]
        approved_tools: Option<ApprovedTools>,
    },
    Command {
        #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
        command_type: Option<String>,
        command: String,
        args: Vec<String>,
        #[serde(rename = "approvedTools", skip_serializing_if = "Option::is_none")]
        approved_tools: Option<ApprovedTools>,
    },
}

#[derive(Serialize, Deserialize)]

struct DuoMcpConfig {
    #[serde(skip)]
    path: PathBuf,

    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, DuoMcpServer>,
}

impl DuoMcpConfig {
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
        let json: DuoMcpConfig = serde_json::from_str(&content)?;

        Ok(Self {
            path,
            mcp_servers: json.mcp_servers,
        })
    }

    pub fn add_server(mut self, name: String, server: DuoMcpServer) -> Self {
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
    let expanded_path = naively_expand_shell_path(mcp_config_path)?;
    let mut config = DuoMcpConfig::get_or_create(expanded_path)?;

    let server_url = format!("http://localhost:{port}/mcp/sse");

    // Check if knowledge graph server already exists
    if let Some(DuoMcpServer::Url {
        command_type,
        url,
        approved_tools,
    }) = config.mcp_servers.get(MCP_NAME)
    {
        // If URL matches and approvedTools already exists (any value), nothing to do
        if url.as_str() == server_url && approved_tools.is_some() && command_type.is_some() {
            return Ok(());
        }

        // If URL matches but approvedTools is missing, add it
        if url.as_str() == server_url {
            let server = DuoMcpServer::Url {
                command_type: Some("sse".to_string()),
                url: url.clone(),
                approved_tools: Some(ApprovedTools::Bool(true)),
            };
            config.mcp_servers.insert(MCP_NAME.to_string(), server);
            config.save()?;
            return Ok(());
        }
    }

    // Server doesn't exist or URL doesn't match, create/update it
    let server = DuoMcpServer::Url {
        command_type: Some("sse".to_string()),
        url: server_url,
        approved_tools: Some(ApprovedTools::Bool(true)),
    };
    config.add_server(MCP_NAME.to_string(), server).save()?;

    Ok(())
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
    if let Some(start) = expanded_path.find('%')
        && let Some(end) = expanded_path[start + 1..].find('%')
    {
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

        let config = DuoMcpConfig::get_or_create(mcp_config_path.clone()).unwrap();
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
        DuoMcpConfig::get_or_create(mcp_config_path.clone())
            .unwrap()
            .add_server(
                "gitlab".to_string(),
                DuoMcpServer::Command {
                    command_type: Some("stdio".to_string()),
                    command: "gitlab".to_string(),
                    args: vec!["mcp".to_string(), "server".to_string()],
                    approved_tools: None,
                },
            )
            .save()
            .unwrap();

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), port).unwrap();

        let config = DuoMcpConfig::get_or_create(mcp_config_path.clone()).unwrap();
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

        let config = DuoMcpConfig::get_or_create(mcp_config_path.clone()).unwrap();
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

        let config = DuoMcpConfig::get_or_create(mcp_config_path.clone()).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);

        check_http_server_is_added_to_existing_config(
            config.mcp_servers.get(MCP_NAME).unwrap(),
            port,
        );

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_updates_file_to_add_approved_tools_and_type_if_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");
        let port = 8080;

        // Create an existing config file with non-pretty json formatting (missing approvedTools)
        fs::write(
            mcp_config_path.clone(),
            "{\"mcpServers\":{\"knowledge-graph\":{\"url\":\"http://localhost:8080/mcp\"}}}",
        )
        .unwrap();

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), port).unwrap();

        // Validate the file has been updated to include approvedTools: true
        let content = fs::read_to_string(mcp_config_path.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let approved = &json["mcpServers"][MCP_NAME]["approvedTools"];
        assert_eq!(approved.as_bool(), Some(true));

        let command_type = &json["mcpServers"][MCP_NAME]["type"];
        assert_eq!(command_type.as_str(), Some("sse"));

        let url = &json["mcpServers"][MCP_NAME]["url"];
        assert_eq!(url.as_str(), Some("http://localhost:8080/mcp/sse"));

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_does_not_update_file_if_approved_tools_already_true() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");
        let port = 8080;

        // Create an existing config file with approvedTools already set to true
        let original_content = r#"{
  "mcpServers": {
    "knowledge-graph": {
      "type": "sse",
      "url": "http://localhost:8080/mcp/sse",
      "approvedTools": true
    }
  }
}"#;
        fs::write(mcp_config_path.clone(), original_content).unwrap();

        // Get original modification time
        let original_modified = fs::metadata(&mcp_config_path).unwrap().modified().unwrap();

        // Small delay to ensure modification time would change if file was written
        std::thread::sleep(std::time::Duration::from_millis(10));

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), port).unwrap();

        // Check that file was not modified (modification time should be the same)
        let new_modified = fs::metadata(&mcp_config_path).unwrap().modified().unwrap();
        assert_eq!(original_modified, new_modified);

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

    fn check_http_server_is_added_to_existing_config(server: &DuoMcpServer, port: u16) {
        match server {
            DuoMcpServer::Url { url, .. } => {
                assert_eq!(*url, format!("http://localhost:{port}/mcp/sse"))
            }
            _ => panic!("Expected URL server"),
        }
    }

    #[test]
    fn test_serializes_approved_tools_as_camel_case() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), 9090).unwrap();

        let content = fs::read_to_string(mcp_config_path.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let approved = &json["mcpServers"][MCP_NAME]["approvedTools"];
        assert_eq!(approved.as_bool(), Some(true));

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_approved_tools_array_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");

        // Create config with array format approvedTools
        let fixture_json = r#"{
            "mcpServers": {
                "test-server": {
                    "url": "https://example.com/mcp",
                    "approvedTools": ["tool1", "tool2"]
                }
            }
        }"#;
        fs::write(mcp_config_path.clone(), fixture_json).unwrap();

        let content = fs::read_to_string(mcp_config_path.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let approved = &json["mcpServers"]["test-server"]["approvedTools"];
        let array = approved.as_array().unwrap();
        assert_eq!(array.len(), 2);
        assert_eq!(array[0].as_str(), Some("tool1"));
        assert_eq!(array[1].as_str(), Some("tool2"));

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_existing_other_server_is_unaffected() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");

        let fixture_json = r#"{
            "mcpServers": {
                "gitlab": {
                    "command": "gitlab",
                    "args": ["mcp", "server"],
                    "approvedTools": true
                },
                "atlassian": {
                    "url": "https://mcp.atlassian.com/v1/sse",
                    "approvedTools": ["tool-name-here"]
                }
            }
        }"#;
        fs::write(mcp_config_path.clone(), fixture_json).unwrap();

        let before_json: serde_json::Value = serde_json::from_str(fixture_json).unwrap();
        let before_gitlab = before_json["mcpServers"]["gitlab"].clone();
        let before_atlassian = before_json["mcpServers"]["atlassian"].clone();

        add_local_http_server_to_mcp_config(mcp_config_path.clone(), 8080).unwrap();

        let after_content = fs::read_to_string(mcp_config_path.clone()).unwrap();
        let after_json: serde_json::Value = serde_json::from_str(&after_content).unwrap();
        let after_gitlab = after_json["mcpServers"]["gitlab"].clone();
        let after_atlassian = after_json["mcpServers"]["atlassian"].clone();

        assert_eq!(before_gitlab, after_gitlab);
        assert_eq!(before_atlassian, after_atlassian);

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_does_not_change_existing_approved_tools() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");
        let port = 8080;
        let server_url = format!("http://localhost:{port}/mcp/sse");

        // Create config with knowledge graph server that has approvedTools set to false
        let fixture_json = format!(
            r#"{{
            "mcpServers": {{
                "{MCP_NAME}": {{
                    "type": "sse",
                    "url": "{server_url}",
                    "approvedTools": false
                }}
            }}
        }}"#
        );
        fs::write(mcp_config_path.clone(), fixture_json).unwrap();

        // Add/update the local server (should NOT change existing approvedTools)
        add_local_http_server_to_mcp_config(mcp_config_path.clone(), port).unwrap();

        // Verify approvedTools remains false (unchanged)
        let content = fs::read_to_string(mcp_config_path.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let approved = &json["mcpServers"][MCP_NAME]["approvedTools"];
        assert_eq!(approved.as_bool(), Some(false)); // Should remain false

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_adds_approved_tools_when_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mcp_config_path = temp_dir.path().join("mcp.json");
        let port = 8080;
        let server_url = format!("http://localhost:{port}/mcp/sse");

        // Create config with knowledge graph server missing approvedTools entirely
        let fixture_json = format!(
            r#"{{
            "mcpServers": {{
                "{MCP_NAME}": {{
                    "url": "{server_url}"
                }}
            }}
        }}"#
        );
        fs::write(mcp_config_path.clone(), fixture_json).unwrap();

        // Add/update the local server (should add approvedTools: true)
        add_local_http_server_to_mcp_config(mcp_config_path.clone(), port).unwrap();

        // Verify approvedTools was added as true
        let content = fs::read_to_string(mcp_config_path.clone()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let approved = &json["mcpServers"][MCP_NAME]["approvedTools"];
        assert_eq!(approved.as_bool(), Some(true));

        temp_dir.close().unwrap();
    }
}
