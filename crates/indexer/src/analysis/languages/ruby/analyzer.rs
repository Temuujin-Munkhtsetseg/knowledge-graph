//! Main Ruby analyzer orchestrating the semantic analysis process.
//!
//! This module implements the central [`RubyAnalyzer`] that coordinates the two-phase
//! Ruby code analysis process, transforming parsed structural data into a semantic
//! Knowledge Graph with accurate cross-references.

use crate::analysis::types::{
    DefinitionNode, DefinitionRelationship, DefinitionType, FileDefinitionRelationship, FqnType,
    SourceLocation,
};
use crate::parsing::processor::{FileProcessingResult, References};
use database::graph::RelationshipType;
use parser_core::{
    references::ReferenceInfo,
    ruby::{
        definitions::RubyDefinitionInfo,
        fqn::ruby_fqn_to_string,
        references::types::{RubyExpressionMetadata, RubyReferenceType, RubyTargetResolution},
        types::{RubyDefinitionType, RubyFqn},
    },
};

use std::collections::HashMap;

// Import the new Ruby-specific analyzers
use super::ExpressionResolver;

pub type RubyReference =
    ReferenceInfo<RubyTargetResolution, RubyReferenceType, RubyExpressionMetadata, RubyFqn>;

pub struct RubyAnalyzer {
    expression_resolver: Option<ExpressionResolver>,
    stats: AnalyzerStats,
}

#[derive(Debug, Default)]
pub struct AnalyzerStats {
    pub definitions_processed: usize,
    pub references_processed: usize,
    pub relationships_created: usize,
}

impl Default for RubyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl RubyAnalyzer {
    pub fn new() -> Self {
        Self {
            expression_resolver: Some(ExpressionResolver::new()),
            stats: AnalyzerStats::default(),
        }
    }

    pub fn get_stats(&self) -> &AnalyzerStats {
        &self.stats
    }

    pub fn process_definitions(
        &mut self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
    ) -> Result<(), String> {
        if let Some(defs) = file_result.definitions.iter_ruby() {
            for definition in defs {
                // Process all definition types including modules for better scope resolution
                // Modules provide namespace context that's important for method resolution

                if let Some((location, ruby_fqn)) =
                    self.create_definition_location(definition, relative_file_path)?
                {
                    let fqn_string = ruby_fqn_to_string(&ruby_fqn);

                    // Create new definition
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.to_string(),
                        DefinitionType::Ruby(definition.definition_type),
                        location.clone(),
                    );

                    let fqn_type = FqnType::Ruby(ruby_fqn.clone());
                    definition_map.insert(
                        (fqn_string.clone(), relative_file_path.to_string()),
                        (definition_node.clone(), fqn_type.clone()),
                    );

                    // Always create file-definition relationship for this specific location
                    file_definition_relationships.push(FileDefinitionRelationship {
                        file_path: relative_file_path.to_string(),
                        definition_fqn: fqn_string.clone(),
                        relationship_type: RelationshipType::FileDefines,
                        definition_location: location.clone(),
                        source_location: None,
                    });

                    // Add definition to expression resolver if available
                    if let Some(ref mut resolver) = self.expression_resolver {
                        resolver.add_definition(fqn_string.clone(), definition_node, &fqn_type);
                    }

                    self.stats.definitions_processed += 1;
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
    ) -> Result<Option<(SourceLocation, RubyFqn)>, String> {
        let location = SourceLocation {
            file_path: file_path.to_string(),
            start_byte: definition.range.byte_offset.0 as i64,
            end_byte: definition.range.byte_offset.1 as i64,
            start_line: definition.range.start.line as i32,
            end_line: definition.range.end.line as i32,
            start_col: definition.range.start.column as i32,
            end_col: definition.range.end.column as i32,
        };

        Ok(Some((location, definition.fqn.clone())))
    }

    /// Create definition-to-definition relationships using definitions map
    pub fn add_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        for ((child_fqn_string, child_file_path), (child_def, child_fqn)) in definition_map {
            // Find parent definition by using FQN parts directly
            if let Some(parent_fqn_string) = self.get_parent_fqn_from_parts(child_fqn)
                && let Some((parent_def, _)) =
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
                        from_location: parent_def.location.clone(),
                        to_location: child_def.location.clone(),
                        relationship_type,
                        source_location: None,
                    });
                }
            }
        }
    }

    /// Processes Ruby references and creates call relationships in the Knowledge Graph.
    pub fn process_references(
        &mut self,
        references: &References,
        file_path: &str,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        if let Some(ref mut resolver) = self.expression_resolver {
            let initial_count = definition_relationships.len();

            resolver.process_references(references, file_path, definition_relationships);

            let new_relationships = definition_relationships.len() - initial_count;

            self.stats.references_processed += new_relationships;
            self.stats.relationships_created += new_relationships;
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
                    .map(|part| part.node_name.to_string())
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
            // Class relationships
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
            // Module relationships
            (DefinitionType::Ruby(Module), DefinitionType::Ruby(Method)) => {
                Some(RelationshipType::ModuleToMethod)
            }
            (DefinitionType::Ruby(Module), DefinitionType::Ruby(SingletonMethod)) => {
                Some(RelationshipType::ModuleToSingletonMethod)
            }
            (DefinitionType::Ruby(Module), DefinitionType::Ruby(Class)) => {
                Some(RelationshipType::ModuleToClass)
            }
            (DefinitionType::Ruby(Module), DefinitionType::Ruby(Module)) => {
                Some(RelationshipType::ModuleToModule)
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
