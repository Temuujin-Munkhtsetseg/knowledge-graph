use database::schema::types::{NodeFieldAccess, NodeTable};

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

impl NodeFieldAccess for ConsolidatedRelationship {
    fn get_u32_field(&self, field_name: &str) -> Option<u32> {
        match field_name {
            "source_id" => self.source_id,
            "target_id" => self.target_id,
            _ => None,
        }
    }

    fn get_u8_field(&self, field_name: &str) -> Option<u8> {
        match field_name {
            "type" => Some(self.relationship_type),
            _ => None,
        }
    }

    fn get_i64_field(&self, field_name: &str) -> Option<i64> {
        match field_name {
            "source_start_byte" => self.start_byte.map(|v| v as i64),
            "source_end_byte" => self.end_byte.map(|v| v as i64),
            _ => None,
        }
    }

    fn get_i32_field(&self, field_name: &str) -> Option<i32> {
        match field_name {
            "source_start_line" => self.start_line.map(|v| v as i32),
            "source_end_line" => self.end_line.map(|v| v as i32),
            "source_start_col" => self.start_column.map(|v| v as i32),
            "source_end_col" => self.end_column.map(|v| v as i32),
            _ => None,
        }
    }
}

// TODO: In a follow-up MR, consolidated relationship struct should either be computed or removed as a concept all together
impl ConsolidatedRelationships {
    pub fn get_relationships_for_pair(
        &self,
        from_table: &NodeTable,
        to_table: &NodeTable,
    ) -> (Option<String>, &Vec<ConsolidatedRelationship>) {
        let filename = from_table.relationship_filename(to_table);
        match (from_table.name, to_table.name) {
            ("DirectoryNode", "DirectoryNode") => (Some(filename), &self.directory_to_directory),
            ("DirectoryNode", "FileNode") => (Some(filename), &self.directory_to_file),
            ("FileNode", "DefinitionNode") => (Some(filename), &self.file_to_definition),
            ("FileNode", "ImportedSymbolNode") => (Some(filename), &self.file_to_imported_symbol),
            ("DefinitionNode", "DefinitionNode") => {
                (Some(filename), &self.definition_to_definition)
            }
            ("DefinitionNode", "ImportedSymbolNode") => {
                (Some(filename), &self.definition_to_imported_symbol)
            }
            ("ImportedSymbolNode", "ImportedSymbolNode") => {
                (Some(filename), &self.imported_symbol_to_imported_symbol)
            }
            ("ImportedSymbolNode", "DefinitionNode") => {
                (Some(filename), &self.imported_symbol_to_definition)
            }
            ("ImportedSymbolNode", "FileNode") => (Some(filename), &self.imported_symbol_to_file),
            _ => {
                static EMPTY_VEC: Vec<ConsolidatedRelationship> = Vec::new();
                (None, &EMPTY_VEC)
            }
        }
    }
}
