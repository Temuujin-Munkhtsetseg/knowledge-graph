use anyhow::Error;
use database::DatabaseResult;
use serde_json::Map;

pub type QueryResult = Box<dyn DatabaseResult>;

pub trait QueryingService: Send + Sync {
    fn execute_query(
        &self,
        project_path: &str,
        query: &str,
        params: Map<String, serde_json::Value>,
    ) -> Result<QueryResult, Error>;
}
