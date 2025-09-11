/// Consolidated relationship data for efficient storage
#[derive(Debug, Clone, Default, Copy)]
pub struct ConsolidatedRelationship {
    pub source_id: Option<u32>,
    pub target_id: Option<u32>,
    pub relationship_type: u8,
    pub start_byte: Option<usize>,
    pub end_byte: Option<usize>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub start_column: Option<usize>,
    pub end_column: Option<usize>,
}

/// Container for different types of consolidated relationships
#[derive(Default, Clone)]
pub struct ConsolidatedRelationships {
    pub directory_to_directory: Vec<ConsolidatedRelationship>,
    pub directory_to_file: Vec<ConsolidatedRelationship>,
    pub file_to_definition: Vec<ConsolidatedRelationship>,
    pub file_to_imported_symbol: Vec<ConsolidatedRelationship>,
    pub definition_to_definition: Vec<ConsolidatedRelationship>,
    pub definition_to_imported_symbol: Vec<ConsolidatedRelationship>,
    pub imported_symbol_to_imported_symbol: Vec<ConsolidatedRelationship>,
    pub imported_symbol_to_definition: Vec<ConsolidatedRelationship>,
    pub imported_symbol_to_file: Vec<ConsolidatedRelationship>,
}
