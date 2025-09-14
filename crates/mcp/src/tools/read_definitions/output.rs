use crate::tools::xml::{ToXml, XmlBuilder};
use serde::Serialize;

#[derive(Serialize)]
pub struct ReadDefinitionsToolOutput {
    pub definitions: Vec<ReadDefinitionsToolDefinitionOutput>,
    pub system_message: String,
}

impl ReadDefinitionsToolOutput {
    pub fn empty(system_message: String) -> Self {
        Self {
            definitions: vec![],
            system_message,
        }
    }
}

#[derive(Serialize)]
pub struct ReadDefinitionsToolDefinitionOutput {
    pub name: String,
    pub fqn: String,
    pub definition_type: String,
    pub location: String,
    pub definition_body: String,
}

impl ToXml for ReadDefinitionsToolOutput {
    fn to_xml(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut builder = XmlBuilder::new();

        builder.start_element("ToolResponse")?;

        builder.start_element("definitions")?;
        for definition in &self.definitions {
            builder.start_element("definition")?;
            builder.write_element("name", &definition.name)?;
            builder.write_element("fqn", &definition.fqn)?;
            builder.write_element("definition-type", &definition.definition_type)?;
            builder.write_element("location", &definition.location)?;
            builder.write_cdata_element("definition-body", &definition.definition_body)?;
            builder.end_element("definition")?;
        }
        builder.end_element("definitions")?;

        builder.write_cdata_element("system-message", &self.system_message)?;

        builder.end_element("ToolResponse")?;
        builder.finish()
    }
}
