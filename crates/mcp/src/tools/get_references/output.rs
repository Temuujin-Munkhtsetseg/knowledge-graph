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
