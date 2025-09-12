use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct GetDefinitionOutput {
    pub definitions: Vec<Definition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum Definition {
    Definition(DefinitionInfo),
    ImportedSymbol(ImportedSymbolInfo),
}

#[derive(Debug, Serialize)]
pub struct DefinitionInfo {
    pub id: String,
    pub name: String,
    pub fqn: String,
    pub primary_file_path: String,
    pub absolute_file_path: String,
    pub start_line: i64,
    pub end_line: i64,
    pub rel_start_col: i64,
    pub rel_end_col: i64,
    pub is_ambiguous: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportedSymbolInfo {
    pub id: String,
    pub name: String,
    pub fqn: String,
    pub primary_file_path: String,
    pub absolute_file_path: String,
    pub start_line: i64,
    pub end_line: i64,
    pub rel_start_col: i64,
    pub rel_end_col: i64,
    pub is_ambiguous: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_error: Option<String>,
}
