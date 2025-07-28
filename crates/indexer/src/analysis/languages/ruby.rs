use crate::analysis::types::{
    DefinitionLocation, DefinitionNode, DefinitionRelationship, DefinitionType,
    FileDefinitionRelationship, FqnType,
};
use crate::parsing::processor::FileProcessingResult;
use database::graph::RelationshipType;
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

    /// Process definitions from a file result and update the definitions map
    pub fn process_definitions(
        &self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
    ) -> Result<(), String> {
        if let Some(defs) = file_result.definitions.iter_ruby() {
            for definition in defs {
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
                        DefinitionType::Ruby(definition.definition_type),
                        location,
                    );
                    definition_map.insert(
                        (fqn_string.clone(), relative_file_path.to_string()),
                        (definition_node, FqnType::Ruby(ruby_fqn)),
                    );

                    // Always create file-definition relationship for this specific location
                    file_definition_relationships.push(FileDefinitionRelationship {
                        file_path: relative_file_path.to_string(),
                        definition_fqn: fqn_string,
                        relationship_type: RelationshipType::FileDefines,
                    });
                }
            }
        }

        Ok(())
    }

    /// Create a definition location from a definition info
    fn create_definition_location(
        &self,
        definition: &RubyDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, RubyFqn)>, String> {
        // Only create definition locations if we have an FQN
        if let Some(ref fqn) = definition.fqn {
            let location = DefinitionLocation {
                file_path: file_path.to_string(),
                start_byte: definition.match_info.range.byte_offset.0 as i64,
                end_byte: definition.match_info.range.byte_offset.1 as i64,
                start_line: definition.match_info.range.start.line as i32,
                end_line: definition.match_info.range.end.line as i32,
                start_col: definition.match_info.range.start.column as i32,
                end_col: definition.match_info.range.end.column as i32,
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

    /// Create definition-to-definition relationships using definitions map
    pub fn add_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        for ((child_fqn_string, child_file_path), (child_def, child_fqn)) in definition_map {
            // Find parent definition by using FQN parts directly
            if let Some(parent_fqn_string) = self.get_parent_fqn_from_parts(child_fqn) {
                if let Some((parent_def, _)) =
                    definition_map.get(&(parent_fqn_string.clone(), child_file_path.clone()))
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
    fn get_parent_fqn_from_parts(&self, fqn: &FqnType) -> Option<String> {
        match fqn {
            FqnType::Ruby(ruby_fqn) => {
                if ruby_fqn.parts.len() <= 1 {
                    // No parent if FQN has only one part or is empty
                    return None;
                }

                // Take all parts except the last one to create parent FQN
                let parent_parts: Vec<String> = ruby_fqn.parts[..ruby_fqn.parts.len() - 1]
                    .iter()
                    .map(|part| part.node_name.clone())
                    .collect();

                if parent_parts.is_empty() {
                    None
                } else {
                    Some(parent_parts.join("::"))
                }
            }
            _ => None,
        }
    }

    /// Determine the relationship type between parent and child definitions using proper types
    fn get_definition_relationship_type(
        &self,
        parent_type: &DefinitionType,
        child_type: &DefinitionType,
    ) -> Option<RelationshipType> {
        use RubyDefinitionType::*;

        match (parent_type, child_type) {
            (DefinitionType::Ruby(Class), DefinitionType::Ruby(Method)) => {
                Some(RelationshipType::ClassToMethod)
            }
            (DefinitionType::Ruby(Class), DefinitionType::Ruby(SingletonMethod)) => {
                Some(RelationshipType::ClassToSingletonMethod)
            }
            (DefinitionType::Ruby(Class), DefinitionType::Ruby(Class)) => {
                Some(RelationshipType::ClassToClass)
            }
            (DefinitionType::Ruby(Class), DefinitionType::Ruby(Lambda)) => {
                Some(RelationshipType::ClassToLambda)
            }
            (DefinitionType::Ruby(Class), DefinitionType::Ruby(Proc)) => {
                Some(RelationshipType::ClassToProc)
            }
            _ => None, // Unknown or unsupported relationship
        }
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
}
