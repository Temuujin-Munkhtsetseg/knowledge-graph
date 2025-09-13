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
