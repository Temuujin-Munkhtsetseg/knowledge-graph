use super::{
    DefinitionLocation, DefinitionNode, DefinitionRelationship, FileDefinitionRelationship,
};
use crate::parsing::processor::FileProcessingResult;
use parser_core::ruby::{
    definitions::RubyDefinitionInfo,
    fqn::{RubyFqn, ruby_fqn_to_string},
    types::RubyDefinitionType,
};
use std::collections::HashMap;

/// Handles Ruby-specific analysis operations
pub struct RubyAnalyzer;

impl Default for RubyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RubyAnalyzer {
    /// Create a new Ruby analyzer
    pub fn new() -> Self {
        Self
    }

    /// Process definitions from a file result and update the merged definitions map
    pub fn process_definitions(
        &self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        definitions_map: &mut HashMap<(String, String), (DefinitionNode, RubyFqn)>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
    ) -> Result<(), String> {
        for definition in &file_result.definitions {
            if definition.definition_type == RubyDefinitionType::Module {
                // Modules are not strictly valid definitions per phase 1, so we skip them for now.
                // TODO: However, we should handle module call definitions eventually.
                continue;
            }

            if let Some((location, ruby_fqn)) =
                self.create_definition_location(definition, relative_file_path)?
            {
                let fqn_string = ruby_fqn_to_string(&ruby_fqn);

                // Create new definition
                let definition_node = DefinitionNode::new(
                    fqn_string.clone(),
                    definition.name.clone(),
                    definition.definition_type,
                    location,
                );
                definitions_map.insert(
                    (fqn_string.clone(), relative_file_path.to_string()),
                    (definition_node, ruby_fqn),
                );

                // Always create file-definition relationship for this specific location
                file_definition_relationships.push(FileDefinitionRelationship {
                    file_path: relative_file_path.to_string(),
                    definition_fqn: fqn_string,
                    relationship_type: "FILE_DEFINES".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Finalize merged definitions and create definition relationships
    pub fn finalize_definitions_and_relationships(
        &self,
        definitions_map: HashMap<(String, String), (DefinitionNode, RubyFqn)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) -> Vec<DefinitionNode> {
        // Extract final definition nodes
        let definition_nodes: Vec<DefinitionNode> = definitions_map
            .values()
            .map(|(def_node, _)| def_node.clone())
            .collect();

        // Create definition-to-definition relationships using merged definitions
        self.create_definition_relationships_from_merged(
            &definitions_map,
            definition_relationships,
        );

        definition_nodes
    }

    /// Create a definition location from a definition info
    fn create_definition_location(
        &self,
        definition: &RubyDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, RubyFqn)>, String> {
        // Only create definition locations if we have an FQN
        if let Some(ref fqn) = definition.fqn {
            let line_number = self.calculate_line_number(definition);

            let location = DefinitionLocation {
                file_path: file_path.to_string(),
                start_byte: definition.match_info.range.byte_offset.0 as i64,
                end_byte: definition.match_info.range.byte_offset.1 as i64,
                line_number,
            };

            Ok(Some((location, fqn.clone())))
        } else {
            // Skip definitions without FQNs
            log::debug!(
                "Skipping definition '{}' without FQN in file '{}'",
                definition.name,
                file_path
            );
            Ok(None)
        }
    }

    /// Create definition-to-definition relationships using merged definitions
    fn create_definition_relationships_from_merged(
        &self,
        definitions_map: &HashMap<(String, String), (DefinitionNode, RubyFqn)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        for ((child_fqn_string, child_file_path), (child_def, child_fqn)) in definitions_map {
            // Find parent definition by using FQN parts directly
            if let Some(parent_fqn_string) = self.get_parent_fqn_from_parts(child_fqn) {
                if let Some((parent_def, _)) =
                    definitions_map.get(&(parent_fqn_string.clone(), child_file_path.clone()))
                {
                    // Determine relationship type based on parent and child types
                    if let Some(relationship_type) = self.get_definition_relationship_type(
                        &parent_def.definition_type,
                        &child_def.definition_type,
                    ) {
                        definition_relationships.push(DefinitionRelationship {
                            from_file_path: parent_def.location.file_path.clone(),
                            to_file_path: child_def.location.file_path.clone(),
                            from_definition_fqn: parent_fqn_string,
                            to_definition_fqn: child_fqn_string.clone(),
                            relationship_type,
                        });
                    }
                }
            }
        }
    }

    /// Extract parent FQN from a RubyFqn by working with parts directly (more efficient)
    fn get_parent_fqn_from_parts(&self, fqn: &RubyFqn) -> Option<String> {
        if fqn.parts.len() <= 1 {
            // No parent if FQN has only one part or is empty
            return None;
        }

        // Take all parts except the last one to create parent FQN
        let parent_parts: Vec<String> = fqn.parts[..fqn.parts.len() - 1]
            .iter()
            .map(|part| part.node_name.clone())
            .collect();

        if parent_parts.is_empty() {
            None
        } else {
            Some(parent_parts.join("::"))
        }
    }

    /// Determine the relationship type between parent and child definitions using proper types
    fn get_definition_relationship_type(
        &self,
        parent_type: &RubyDefinitionType,
        child_type: &RubyDefinitionType,
    ) -> Option<String> {
        use RubyDefinitionType::*;

        match (parent_type, child_type) {
            (Class, Method) => Some("CLASS_TO_METHOD".to_string()),
            (Class, SingletonMethod) => Some("CLASS_TO_SINGLETON_METHOD".to_string()),
            (Class, Class) => Some("CLASS_TO_CLASS".to_string()),
            (Class, Lambda) => Some("CLASS_TO_LAMBDA".to_string()),
            (Class, Proc) => Some("CLASS_TO_PROC".to_string()),
            _ => None, // Unknown or unsupported relationship
        }
    }

    /// Calculate approximate line number from byte position
    fn calculate_line_number(&self, definition: &RubyDefinitionInfo) -> i32 {
        // Use the line number from the match info (1-indexed)
        definition.match_info.range.start.line as i32
    }

    /// Get the Ruby-specific scope relationship between definitions
    pub fn get_ruby_scope_relationship(
        parent_type: &RubyDefinitionType,
        child_type: &RubyDefinitionType,
    ) -> Option<String> {
        use RubyDefinitionType::*;

        match (parent_type, child_type) {
            // Namespace relationships
            (Module, _) => Some("NAMESPACE".to_string()),
            (Class, Method | SingletonMethod) => Some("SCOPE".to_string()),
            // Block relationships - Note: Block is not a RubyDefinitionType, so this pattern won't work
            (Method | SingletonMethod, _) => Some("BLOCK_SCOPE".to_string()),
            _ => None,
        }
    }

    /// Determine if a definition is at the top level (no namespace)
    pub fn is_top_level_definition(fqn: &RubyFqn) -> bool {
        fqn.parts.len() == 1
    }

    /// Extract module or class names from an FQN
    pub fn extract_namespace_parts(fqn: &RubyFqn) -> Vec<String> {
        fqn.parts[..fqn.parts.len().saturating_sub(1)]
            .iter()
            .map(|part| part.node_name.clone())
            .collect()
    }

    /// Get the definition name (last part of FQN)
    pub fn get_definition_name(fqn: &RubyFqn) -> Option<String> {
        fqn.parts.last().map(|part| part.node_name.clone())
    }
}
