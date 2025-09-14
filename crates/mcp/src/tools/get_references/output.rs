use crate::tools::xml::{ToXml, XmlBuilder};
use serde::Serialize;

#[derive(Serialize)]
pub struct GetReferencesToolOutput {
    pub definitions: Vec<GetReferencesToolDefinitionOutput>,
    pub next_page: Option<u64>,
    pub system_message: String,
}

impl GetReferencesToolOutput {
    pub fn empty(system_message: String) -> Self {
        Self {
            definitions: vec![],
            next_page: None,
            system_message,
        }
    }
}

#[derive(Serialize)]
pub struct GetReferencesToolDefinitionOutput {
    pub name: String,
    pub location: String,
    pub definition_type: String,
    pub fqn: String,
    pub references: Vec<GetReferencesToolReferenceOutput>,
}

#[derive(Serialize)]
pub struct GetReferencesToolReferenceOutput {
    pub reference_type: String,
    pub location: String,
    pub context: String,
}

#[derive(Serialize)]
pub struct GetReferencesToolSummaryOutput {
    pub total_found: u64,
    pub total_returned: u64,
    pub has_more: bool,
}

impl ToXml for GetReferencesToolOutput {
    fn to_xml(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut builder = XmlBuilder::new();

        builder.start_element("ToolResponse")?;

        builder.start_element("definitions")?;
        for definition in &self.definitions {
            builder.start_element("definition")?;
            builder.write_element("name", &definition.name)?;
            builder.write_element("location", &definition.location)?;
            builder.write_element("definition-type", &definition.definition_type)?;
            builder.write_element("fqn", &definition.fqn)?;

            builder.start_element("references")?;
            for reference in &definition.references {
                builder.start_element("reference")?;
                builder.write_element("reference-type", &reference.reference_type)?;
                builder.write_element("location", &reference.location)?;
                builder.write_cdata_element("context", &reference.context)?;
                builder.end_element("reference")?;
            }
            builder.end_element("references")?;

            builder.end_element("definition")?;
        }
        builder.end_element("definitions")?;

        builder.write_optional_numeric_element("next-page", &self.next_page)?;
        builder.write_cdata_element("system-message", &self.system_message)?;

        builder.end_element("ToolResponse")?;
        builder.finish()
    }
}
