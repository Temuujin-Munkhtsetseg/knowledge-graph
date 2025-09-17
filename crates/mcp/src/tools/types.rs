use async_trait::async_trait;
use rmcp::model::CallToolResult;
use rmcp::model::{JsonObject, Tool};

#[async_trait]
pub trait KnowledgeGraphTool: Send + Sync {
    fn name(&self) -> &str;
    fn to_mcp_tool(&self) -> Tool;
    async fn call(&self, params: JsonObject) -> Result<CallToolResult, rmcp::ErrorData>;
}

pub struct KnowledgeGraphToolInput {
    pub params: JsonObject,
}

impl KnowledgeGraphToolInput {
    pub fn get_string_array(&self, key: &str) -> Result<Vec<String>, rmcp::ErrorData> {
        let array = self
            .params
            .get(key)
            .and_then(|v| v.as_array())
            .ok_or(rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                format!("Missing array parameter: {}", key),
                None,
            ))?;

        Ok(array
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect())
    }

    pub fn get_string(&self, key: &str) -> Result<&str, rmcp::ErrorData> {
        self.params
            .get(key)
            .and_then(|v| v.as_str())
            .ok_or(rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                format!("Missing string parameter: {}", key),
                None,
            ))
    }

    pub fn get_usize(&self, key: &str) -> Result<usize, rmcp::ErrorData> {
        self.params
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .ok_or(rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                format!("Missing usize parameter: {}", key),
                None,
            ))
    }

    pub fn get_u64(&self, key: &str) -> Result<u64, rmcp::ErrorData> {
        self.params
            .get(key)
            .and_then(|v| v.as_u64())
            .ok_or(rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                format!("Missing u64 parameter: {}", key),
                None,
            ))
    }

    pub fn get_boolean(&self, key: &str) -> Result<bool, rmcp::ErrorData> {
        self.params
            .get(key)
            .and_then(|v| v.as_bool())
            .ok_or(rmcp::ErrorData::new(
                rmcp::model::ErrorCode::INVALID_REQUEST,
                format!("Missing boolean parameter: {}", key),
                None,
            ))
    }

    // Optional parameter methods that return None if the parameter is missing
    pub fn get_u64_optional(&self, key: &str) -> Option<u64> {
        self.params.get(key).and_then(|v| v.as_u64())
    }

    pub fn get_usize_optional(&self, key: &str) -> Option<usize> {
        self.params
            .get(key)
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
    }

    pub fn get_boolean_optional(&self, key: &str) -> Option<bool> {
        self.params.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_string_array_optional(&self, key: &str) -> Option<Vec<String>> {
        self.params
            .get(key)
            .and_then(|v| v.as_array())
            .map(|array| {
                array
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
    }
}
