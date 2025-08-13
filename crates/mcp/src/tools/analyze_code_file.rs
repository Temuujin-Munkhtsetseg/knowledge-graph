use std::{borrow::Cow, sync::Arc};

use database::querying::{QueryLibrary, QueryingService};
use rmcp::model::{CallToolResult, Content, ErrorCode, JsonObject, Tool};
use serde_json::{Map, Value};
use std::path::PathBuf;
use workspace_manager::WorkspaceManager;

use crate::tools::types::KnowledgeGraphTool;

pub const ANALYZE_CODE_FILE_TOOL_NAME: &str = "analyze_code_file";
const ANALYZE_CODE_FILE_TOOL_DESCRIPTION: &str = "Analyzes the structure and dependencies of code files. \
Lists all class/function definitions, imports, exports, and key components. \
Essential for understanding file organization, dependencies, and codebase architecture before making modifications or providing guidance.";

pub struct AnalyzeCodeFileTool {
    pub query_service: Arc<dyn QueryingService>,
    pub workspace_manager: Arc<WorkspaceManager>,
}

impl AnalyzeCodeFileTool {
    pub fn new(
        query_service: Arc<dyn QueryingService>,
        workspace_manager: Arc<WorkspaceManager>,
    ) -> Self {
        Self {
            query_service,
            workspace_manager,
        }
    }

    fn get_definitions(
        &self,
        database_path: PathBuf,
        file_path: &str,
        limit: u64,
    ) -> Result<Value, rmcp::ErrorData> {
        let def_query = QueryLibrary::get_file_definitions_query();
        let mut def_params = Map::new();
        def_params.insert(
            "file_path".to_string(),
            Value::String(file_path.to_string()),
        );
        def_params.insert("limit".to_string(), Value::Number(limit.into()));

        let mut def_result = self
            .query_service
            .execute_query(database_path, def_query.query, def_params)
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    format!("Could not execute definitions query: {e}."),
                    None,
                )
            })?;

        def_result.to_json(&def_query.result).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                format!("Could not serialize definitions: {e}."),
                None,
            )
        })
    }

    fn get_imports(
        &self,
        database_path: PathBuf,
        file_path: &str,
        limit: u64,
    ) -> Result<Value, rmcp::ErrorData> {
        let imp_query = QueryLibrary::get_file_imports_query();
        let mut imp_params = Map::new();
        imp_params.insert(
            "file_path".to_string(),
            Value::String(file_path.to_string()),
        );
        imp_params.insert("limit".to_string(), Value::Number(limit.into()));

        let mut imp_result = self
            .query_service
            .execute_query(database_path, imp_query.query, imp_params)
            .map_err(|e| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    format!("Could not execute imports query: {e}."),
                    None,
                )
            })?;

        imp_result.to_json(&imp_query.result).map_err(|e| {
            rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                format!("Could not serialize imports: {e}."),
                None,
            )
        })
    }

    fn create_repomap_json(&self, definitions: &Value, imports: &Value) -> Value {
        // Map imports to only include the fields used previously plus location
        let mapped_imports = if let Some(imps) = imports.as_array() {
            let mut result = Vec::with_capacity(imps.len());
            for imp in imps {
                if let Some(obj) = imp.as_object() {
                    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let import_path = obj
                        .get("import_path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let alias = obj.get("alias").and_then(|v| v.as_str()).unwrap_or("");
                    let line = obj.get("line_number").and_then(|v| v.as_u64()).unwrap_or(0);

                    let mut mapped = JsonObject::new();
                    mapped.insert("name".to_string(), Value::String(name.to_string()));
                    mapped.insert(
                        "import_path".to_string(),
                        Value::String(import_path.to_string()),
                    );
                    mapped.insert("alias".to_string(), Value::String(alias.to_string()));

                    let mut location = JsonObject::new();
                    location.insert("line".to_string(), Value::Number(line.into()));
                    mapped.insert("location".to_string(), Value::Object(location));

                    result.push(Value::Object(mapped));
                }
            }
            Value::Array(result)
        } else {
            Value::Array(vec![])
        };

        // Map definitions to only include the fields used previously plus location
        let mapped_definitions = if let Some(defs) = definitions.as_array() {
            let mut result = Vec::with_capacity(defs.len());
            for def in defs {
                if let Some(obj) = def.as_object() {
                    let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let fqn = obj.get("fqn").and_then(|v| v.as_str()).unwrap_or("");
                    let definition_type = obj
                        .get("definition_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN");
                    let line = obj.get("line_number").and_then(|v| v.as_u64()).unwrap_or(0);

                    let mut mapped = JsonObject::new();
                    mapped.insert("name".to_string(), Value::String(name.to_string()));
                    mapped.insert("fqn".to_string(), Value::String(fqn.to_string()));
                    mapped.insert(
                        "definition_type".to_string(),
                        Value::String(definition_type.to_string()),
                    );

                    let mut location = JsonObject::new();
                    location.insert("line".to_string(), Value::Number(line.into()));
                    mapped.insert("location".to_string(), Value::Object(location));

                    result.push(Value::Object(mapped));
                }
            }
            Value::Array(result)
        } else {
            Value::Array(vec![])
        };

        let mut repomap_obj = JsonObject::new();
        repomap_obj.insert("imports".to_string(), mapped_imports);
        repomap_obj.insert("definitions".to_string(), mapped_definitions);
        Value::Object(repomap_obj)
    }
}

impl KnowledgeGraphTool for AnalyzeCodeFileTool {
    fn name(&self) -> &str {
        ANALYZE_CODE_FILE_TOOL_NAME
    }

    fn to_mcp_tool(&self) -> Tool {
        let mut properties = JsonObject::new();

        let mut project_property = JsonObject::new();
        project_property.insert("type".to_string(), Value::String("string".to_string()));
        project_property.insert(
            "description".to_string(),
            Value::String("The absolute path to the current project directory.".to_string()),
        );
        properties.insert(
            "project_absolute_path".to_string(),
            Value::Object(project_property),
        );

        let mut files_property = JsonObject::new();
        files_property.insert("type".to_string(), Value::String("array".to_string()));
        files_property.insert(
            "description".to_string(),
            Value::String("The absolute paths to the files to analyze.".to_string()),
        );
        let mut file_items = JsonObject::new();
        file_items.insert("type".to_string(), Value::String("string".to_string()));
        files_property.insert("items".to_string(), Value::Object(file_items));
        properties.insert("files".to_string(), Value::Object(files_property));

        let mut input_schema = JsonObject::new();
        input_schema.insert("type".to_string(), Value::String("object".to_string()));
        input_schema.insert("properties".to_string(), Value::Object(properties));
        input_schema.insert(
            "required".to_string(),
            Value::Array(vec![
                Value::String("project_absolute_path".to_string()),
                Value::String("files".to_string()),
            ]),
        );

        Tool {
            name: Cow::Borrowed(ANALYZE_CODE_FILE_TOOL_NAME),
            description: Some(Cow::Borrowed(ANALYZE_CODE_FILE_TOOL_DESCRIPTION)),
            input_schema: Arc::new(input_schema),
            output_schema: None,
            annotations: None,
        }
    }

    fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData> {
        let project_absolute_path = params.get("project_absolute_path").and_then(|v| v.as_str());
        if project_absolute_path.is_none() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Missing project_absolute_path".to_string(),
                None,
            ));
        }
        let project_absolute_path = project_absolute_path.unwrap();

        let database_path = self
            .workspace_manager
            .get_project_for_path(project_absolute_path)
            .map(|p| p.database_path);
        if database_path.is_none() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Project not found in workspace manager".to_string(),
                None,
            ));
        }
        let database_path = database_path.unwrap();

        let files = params.get("files").and_then(|v| v.as_array());
        if files.is_none() {
            return Err(rmcp::ErrorData::new(
                ErrorCode::INVALID_REQUEST,
                "Missing files".to_string(),
                None,
            ));
        }
        let files = files.unwrap();
        let limit = 100;

        let mut results = Vec::new();

        for file_path_val in files {
            if let Some(file_path) = file_path_val.as_str() {
                let definitions = self.get_definitions(database_path.clone(), file_path, limit)?;
                let imports = self.get_imports(database_path.clone(), file_path, limit)?;

                let repomap_json = self.create_repomap_json(&definitions, &imports);

                let mut file_result = JsonObject::new();
                file_result.insert("file".to_string(), Value::String(file_path.to_string()));
                file_result.insert("repomap".to_string(), repomap_json);
                results.push(Content::json(file_result).unwrap());
            }
        }

        Ok(CallToolResult::success(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use database::testing::MockQueryingService;
    use std::sync::Arc;
    use tempfile::TempDir;
    use testing::repository::TestRepository;

    fn create_test_workspace_manager() -> (Arc<WorkspaceManager>, String) {
        let temp_workspace_dir = TempDir::new().unwrap();
        let workspace_path = temp_workspace_dir.path().join("test_workspace");
        std::fs::create_dir_all(&workspace_path).unwrap();

        let test_project_path = workspace_path.join("test_project");
        TestRepository::new(&test_project_path, Some("test-repo"));

        let temp_data_dir = TempDir::new().unwrap();
        let manager = Arc::new(
            WorkspaceManager::new_with_directory(temp_data_dir.path().to_path_buf()).unwrap(),
        );

        manager.register_workspace_folder(&workspace_path).unwrap();

        let projects = manager.list_all_projects();
        let project_path = projects[0].project_path.clone();

        (manager, project_path)
    }

    #[test]
    fn test_analyze_code_file_creates_repomap() {
        let file_path = "app/models/user_model.rb";
        let definitions = vec![
            vec![
                "app.models.user_model.UserModel".to_string(),
                "UserModel".to_string(),
                "class".to_string(),
                "4".to_string(),
                file_path.to_string(),
            ],
            vec![
                "app.models.user_model.UserModel.full_name".to_string(),
                "full_name".to_string(),
                "method".to_string(),
                "5".to_string(),
                file_path.to_string(),
            ],
        ];

        let imports = vec![vec![
            "BaseModel".to_string(),
            "base_model".to_string(),
            "BM".to_string(),
            "1".to_string(),
        ]];

        let mock_query_service = MockQueryingService::new()
            .with_return_data(
                vec![
                    "name".to_string(),
                    "import_path".to_string(),
                    "alias".to_string(),
                    "line_number".to_string(),
                ],
                imports,
            )
            .with_return_data(
                vec![
                    "fqn".to_string(),
                    "name".to_string(),
                    "definition_type".to_string(),
                    "line_number".to_string(),
                    "file_path".to_string(),
                ],
                definitions,
            );

        let (workspace_manager, project_path) = create_test_workspace_manager();
        let tool = AnalyzeCodeFileTool::new(Arc::new(mock_query_service), workspace_manager);

        let mut params = JsonObject::new();
        params.insert(
            "project_absolute_path".to_string(),
            Value::String(project_path),
        );
        params.insert(
            "files".to_string(),
            Value::Array(vec![Value::String(file_path.to_string())]),
        );

        let result = tool.call(params).unwrap();
        let text = result.content.unwrap()[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .clone();

        let obj: Value = serde_json::from_str(&text).unwrap();

        // Validate file field
        assert_eq!(obj.get("file").and_then(|v| v.as_str()).unwrap(), file_path);

        // Validate repomap structure
        let repomap = obj
            .get("repomap")
            .and_then(|v| v.as_object())
            .expect("repomap should be an object");

        // Validate imports
        let imports = repomap
            .get("imports")
            .and_then(|v| v.as_array())
            .expect("imports should be an array");
        assert_eq!(imports.len(), 1);
        let imp = imports[0].as_object().expect("import should be object");
        assert_eq!(
            imp.get("name").and_then(|v| v.as_str()).unwrap(),
            "BaseModel"
        );
        assert_eq!(
            imp.get("import_path").and_then(|v| v.as_str()).unwrap(),
            "base_model"
        );
        assert_eq!(imp.get("alias").and_then(|v| v.as_str()).unwrap(), "BM");
        let imp_loc = imp
            .get("location")
            .and_then(|v| v.as_object())
            .expect("import should have location");
        assert_eq!(imp_loc.get("line").and_then(|v| v.as_u64()).unwrap(), 1);

        // Validate definitions
        let definitions = repomap
            .get("definitions")
            .and_then(|v| v.as_array())
            .expect("definitions should be an array");

        let has_class = definitions.iter().any(|d| {
            let o = d.as_object().unwrap();
            let loc = o.get("location").and_then(|v| v.as_object()).unwrap();
            o.get("definition_type").and_then(|v| v.as_str()) == Some("class")
                && o.get("name").and_then(|v| v.as_str()) == Some("UserModel")
                && o.get("fqn").and_then(|v| v.as_str()) == Some("app.models.user_model.UserModel")
                && loc.get("line").and_then(|v| v.as_u64()) == Some(4)
        });
        assert!(has_class);

        let has_method = definitions.iter().any(|d| {
            let o = d.as_object().unwrap();
            let loc = o.get("location").and_then(|v| v.as_object()).unwrap();
            o.get("definition_type").and_then(|v| v.as_str()) == Some("method")
                && o.get("name").and_then(|v| v.as_str()) == Some("full_name")
                && o.get("fqn").and_then(|v| v.as_str())
                    == Some("app.models.user_model.UserModel.full_name")
                && loc.get("line").and_then(|v| v.as_u64()) == Some(5)
        });
        assert!(has_method);
    }
}
