use rmcp::model::{ErrorCode, JsonObject};

pub struct GetDefinitionInput {
    pub file_path: String,
    pub line: String,
    pub symbol_name: String,
}

impl TryFrom<JsonObject> for GetDefinitionInput {
    type Error = rmcp::ErrorData;

    fn try_from(params: JsonObject) -> Result<Self, Self::Error> {
        let file_path = params
            .get("absolute_file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Missing absolute_file_path".to_string(),
                    None,
                )
            })?
            .to_string();

        let line = params
            .get("line")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                rmcp::ErrorData::new(ErrorCode::INVALID_REQUEST, "Missing line".to_string(), None)
            })?
            .to_string();

        let symbol_name = params
            .get("symbol_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                rmcp::ErrorData::new(
                    ErrorCode::INVALID_REQUEST,
                    "Missing symbol_name".to_string(),
                    None,
                )
            })?
            .to_string();

        Ok(Self {
            file_path,
            line,
            symbol_name,
        })
    }
}
