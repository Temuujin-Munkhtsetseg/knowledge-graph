use rmcp::model::{ErrorCode, JsonObject};

#[derive(Debug, Clone)]
pub struct RepoMapInput {
    pub project_absolute_path: String,
    pub relative_paths: Vec<String>,
    pub depth: u64,
    pub show_directories: bool,
    pub show_definitions: bool,
    pub page: u64,
    pub page_size: u64,
}

impl TryFrom<JsonObject> for RepoMapInput {
    type Error = rmcp::ErrorData;

    fn try_from(params: JsonObject) -> Result<Self, Self::Error> {
        let project_absolute_path = params
            .get("project_absolute_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Missing project_absolute_path".to_string(),
                    None,
                )
            })?
            .to_string();

        let relative_paths = params
            .get("relative_paths")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Missing relative_paths".to_string(),
                    None,
                )
            })?
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        let depth = params
            .get("depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .max(1);
        let show_directories = params
            .get("show_directories")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let show_definitions = params
            .get("show_definitions")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let page = params
            .get("page")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .max(1);
        let page_size = params
            .get("page_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(200)
            .max(1);

        Ok(Self {
            project_absolute_path,
            relative_paths,
            depth,
            show_directories,
            show_definitions,
            page,
            page_size,
        })
    }
}
