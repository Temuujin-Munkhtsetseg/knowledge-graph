use crate::analysis::types::{
    DefinitionLocation, DefinitionNode, DefinitionRelationship, DefinitionType,
    FileDefinitionRelationship, FqnType,
};
use crate::parsing::processor::FileProcessingResult;
use database::graph::RelationshipType;
use parser_core::rust::{
    fqn::rust_fqn_to_string,
    types::{RustDefinitionInfo, RustDefinitionType, RustFqn},
};
use smallvec::SmallVec;
use std::collections::HashMap;

// Handles Rust-specific analysis operations
pub struct RustAnalyzer;

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalyzer {
    /// Create a new Rust analyzer
    pub fn new() -> Self {
        Self
    }

    /// Process definitions from a file result and update the definitions map
    pub fn process_definitions(
        &self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
    ) {
        if let Some(defs) = file_result.definitions.iter_rust() {
            for definition in defs {
                if let Ok(Some((location, fqn))) =
                    self.create_definition_location(definition, relative_file_path)
                {
                    let fqn_string = rust_fqn_to_string(&fqn);
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.clone(),
                        DefinitionType::Rust(definition.definition_type),
                        location.clone(),
                    );

                    let key = (fqn_string, relative_file_path.to_string());

                    if definition_map.contains_key(&key) {
                        log::warn!(
                            "Duplicate definition found for Rust: {} in file {}",
                            definition.name,
                            relative_file_path
                        );
                        continue;
                    }

                    definition_map
                        .insert(key, (definition_node.clone(), FqnType::Rust(fqn.clone())));

                    if self.is_top_level_definition(&fqn) {
                        file_definition_relationships.push(FileDefinitionRelationship {
                            file_path: relative_file_path.to_string(),
                            definition_fqn: definition_node.fqn.clone(),
                            relationship_type: RelationshipType::FileDefines,
                            definition_location: location.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Add definition relationships for Rust
    pub fn add_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        self.add_rust_definition_relationships(definition_map, definition_relationships);
    }

    /// Create definition location from Rust definition info
    fn create_definition_location(
        &self,
        definition: &RustDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, RustFqn)>, String> {
        let location = DefinitionLocation {
            file_path: file_path.to_string(),
            start_line: definition.range.start.line as i32,
            start_col: definition.range.start.column as i32,
            end_line: definition.range.end.line as i32,
            end_col: definition.range.end.column as i32,
            start_byte: definition.range.byte_offset.0 as i64,
            end_byte: definition.range.byte_offset.1 as i64,
        };

        Ok(Some((location, definition.fqn.clone())))
    }

    /// Add Rust-specific definition relationships
    fn add_rust_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        let rust_definitions: Vec<_> = definition_map
            .values()
            .filter_map(|(node, fqn_type)| {
                if let FqnType::Rust(fqn) = fqn_type {
                    Some((node, fqn))
                } else {
                    None
                }
            })
            .collect();

        for (node, fqn) in &rust_definitions {
            self.create_rust_nested_relationships(
                node,
                fqn,
                &rust_definitions,
                definition_relationships,
            );
        }
    }

    /// Create nested relationships for Rust definitions (e.g., module contains struct, impl contains method)
    fn create_rust_nested_relationships(
        &self,
        node: &DefinitionNode,
        fqn: &RustFqn,
        all_definitions: &[(&DefinitionNode, &RustFqn)],
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        if fqn.len() <= 1 {
            return; // No parent for top-level definitions
        }

        // Look for parent definitions in the FQN hierarchy
        for i in 1..fqn.len() {
            let parent_parts = fqn.parts[..fqn.len() - i].to_vec();
            let parent_fqn = RustFqn::new(SmallVec::from_vec(parent_parts));
            let parent_fqn_string = rust_fqn_to_string(&parent_fqn);

            // Find the parent definition
            if let Some((parent_node, _)) = all_definitions
                .iter()
                .find(|(def_node, _)| def_node.fqn == parent_fqn_string)
            {
                let relationship_type = self.determine_rust_relationship_type(
                    &parent_node.definition_type,
                    &node.definition_type,
                );

                if let Some(rel_type) = relationship_type {
                    // For now, simplify by using a generic definition relationship
                    definition_relationships.push(DefinitionRelationship {
                        from_definition_fqn: parent_node.fqn.clone(),
                        to_definition_fqn: node.fqn.clone(),
                        from_file_path: parent_node.location.file_path.clone(),
                        from_location: parent_node.location.clone(),
                        to_file_path: node.location.file_path.clone(),
                        to_location: node.location.clone(),
                        relationship_type: rel_type,
                    });
                    break; // Only create relationship with immediate parent
                }
            }
        }
    }

    /// Determine the appropriate relationship type between Rust definitions
    fn determine_rust_relationship_type(
        &self,
        parent_type: &DefinitionType,
        child_type: &DefinitionType,
    ) -> Option<RelationshipType> {
        match (parent_type, child_type) {
            // Use appropriate relationship types based on what's available
            (DefinitionType::Rust(RustDefinitionType::Module), _) => {
                Some(RelationshipType::ModuleToSingletonMethod)
            }
            (
                DefinitionType::Rust(RustDefinitionType::Struct),
                DefinitionType::Rust(RustDefinitionType::Field),
            ) => {
                Some(RelationshipType::ClassToMethod) // Reuse for struct->field
            }
            (
                DefinitionType::Rust(RustDefinitionType::Enum),
                DefinitionType::Rust(RustDefinitionType::Variant),
            ) => {
                Some(RelationshipType::ClassToMethod) // Reuse for enum->variant
            }
            (
                DefinitionType::Rust(RustDefinitionType::Trait),
                DefinitionType::Rust(RustDefinitionType::Method),
            ) => {
                Some(RelationshipType::ClassToMethod) // Reuse for trait->method
            }
            (
                DefinitionType::Rust(RustDefinitionType::Impl),
                DefinitionType::Rust(RustDefinitionType::Method),
            ) => {
                Some(RelationshipType::ClassToMethod) // Reuse for impl->method
            }
            (
                DefinitionType::Rust(RustDefinitionType::Impl),
                DefinitionType::Rust(RustDefinitionType::AssociatedFunction),
            ) => {
                Some(RelationshipType::ClassToMethod) // Reuse for impl->associated function
            }
            (
                DefinitionType::Rust(RustDefinitionType::Union),
                DefinitionType::Rust(RustDefinitionType::Field),
            ) => {
                Some(RelationshipType::ClassToMethod) // Reuse for union->field
            }
            _ => None,
        }
    }

    fn is_top_level_definition(&self, fqn: &RustFqn) -> bool {
        fqn.len() == 1
    }
}
