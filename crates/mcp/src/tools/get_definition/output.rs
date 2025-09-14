use crate::tools::xml::{ToXml, XmlBuilder};
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

impl ToXml for GetDefinitionOutput {
    fn to_xml(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut builder = XmlBuilder::new();

        builder.start_element("ToolResponse")?;

        builder.start_element("definitions")?;
        for definition in &self.definitions {
            match definition {
                Definition::Definition(def_info) => {
                    builder.start_element("definition")?;
                    builder.write_element("type", "Definition")?;
                    builder.write_element("id", &def_info.id)?;
                    builder.write_element("name", &def_info.name)?;
                    builder.write_element("fqn", &def_info.fqn)?;
                    builder.write_element("primary-file-path", &def_info.primary_file_path)?;
                    builder.write_element("absolute-file-path", &def_info.absolute_file_path)?;
                    builder.write_numeric_element("start-line", def_info.start_line)?;
                    builder.write_numeric_element("end-line", def_info.end_line)?;
                    builder.write_numeric_element("rel-start-col", def_info.rel_start_col)?;
                    builder.write_numeric_element("rel-end-col", def_info.rel_end_col)?;
                    builder.write_boolean_element("is-ambiguous", def_info.is_ambiguous)?;
                    builder.write_optional_cdata_element("code", &def_info.code)?;
                    builder.write_optional_element("code-error", &def_info.code_error)?;
                    builder.end_element("definition")?;
                }
                Definition::ImportedSymbol(symbol_info) => {
                    builder.start_element("definition")?;
                    builder.write_element("type", "ImportedSymbol")?;
                    builder.write_element("id", &symbol_info.id)?;
                    builder.write_element("name", &symbol_info.name)?;
                    builder.write_element("fqn", &symbol_info.fqn)?;
                    builder.write_element("primary-file-path", &symbol_info.primary_file_path)?;
                    builder.write_element("absolute-file-path", &symbol_info.absolute_file_path)?;
                    builder.write_numeric_element("start-line", symbol_info.start_line)?;
                    builder.write_numeric_element("end-line", symbol_info.end_line)?;
                    builder.write_numeric_element("rel-start-col", symbol_info.rel_start_col)?;
                    builder.write_numeric_element("rel-end-col", symbol_info.rel_end_col)?;
                    builder.write_boolean_element("is-ambiguous", symbol_info.is_ambiguous)?;
                    builder.write_optional_cdata_element("code", &symbol_info.code)?;
                    builder.write_optional_element("code-error", &symbol_info.code_error)?;
                    builder.end_element("definition")?;
                }
            }
        }
        builder.end_element("definitions")?;

        builder.write_optional_cdata_element("system-message", &self.system_message)?;

        builder.end_element("ToolResponse")?;
        builder.finish()
    }
}
