use crate::analysis::types::{
    DefinitionLocation, DefinitionNode, DefinitionRelationship, DefinitionType,
    FileDefinitionRelationship, FqnType,
};
use crate::parsing::processor::FileProcessingResult;
use parser_core::python::{
    fqn::python_fqn_to_string,
    types::{PythonDefinitionInfo, PythonDefinitionType, PythonFqn},
};
use std::collections::HashMap;

/// Handles Python-specific analysis operations
pub struct PythonAnalyzer;

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonAnalyzer {
    /// Create a new Python analyzer
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
        if let Some(definitions) = &file_result.definitions {
            if let Some(defs) = definitions.iter_python() {
                for definition in defs {
                    if let Ok(Some((location, fqn))) =
                        self.create_definition_location(definition, relative_file_path)
                    {
                        let fqn_string = python_fqn_to_string(&fqn);
                        let definition_node = DefinitionNode::new(
                            fqn_string.clone(),
                            definition.name.clone(),
                            DefinitionType::Python(definition.definition_type),
                            location,
                        );
                        definition_map.insert(
                            (fqn_string.clone(), relative_file_path.to_string()),
                            (definition_node, FqnType::Python(fqn)),
                        );
                        file_definition_relationships.push(FileDefinitionRelationship {
                            file_path: relative_file_path.to_string(),
                            definition_fqn: fqn_string,
                            relationship_type: "FILE_DEFINES".to_string(),
                        });
                    }
                }
            }
        }
    }

    /// Create a definition location from a definition info
    fn create_definition_location(
        &self,
        definition: &PythonDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, PythonFqn)>, String> {
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

    /// Create definition-to-definition relationships using definitions map
    pub fn add_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        for ((child_fqn_string, child_file_path), (child_def, child_fqn)) in definition_map {
            if let Some(parent_fqn_string) = self.get_parent_fqn_string(child_fqn) {
                if let Some((parent_def, _)) =
                    definition_map.get(&(parent_fqn_string.clone(), child_file_path.to_string()))
                {
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

    /// Extract parent FQN string from a given FQN
    fn get_parent_fqn_string(&self, fqn: &FqnType) -> Option<String> {
        match fqn {
            FqnType::Python(python_fqn) => {
                let parts_len = python_fqn.parts.len();
                if parts_len <= 1 {
                    return None;
                }

                // Build parent string directly without creating new Vec
                Some(
                    python_fqn.parts[..parts_len - 1]
                        .iter()
                        .map(|part| part.node_name.replace('.', "#"))
                        .collect::<Vec<_>>()
                        .join("."),
                )
            }
            _ => None,
        }
    }

    fn simplify_definition_type(&self, definition_type: &DefinitionType) -> Option<DefinitionType> {
        use PythonDefinitionType::*;

        match definition_type {
            DefinitionType::Python(Class) | DefinitionType::Python(DecoratedClass) => {
                Some(DefinitionType::Python(Class))
            }
            DefinitionType::Python(Method)
            | DefinitionType::Python(AsyncMethod)
            | DefinitionType::Python(DecoratedMethod)
            | DefinitionType::Python(DecoratedAsyncMethod) => Some(DefinitionType::Python(Method)),
            DefinitionType::Python(Function)
            | DefinitionType::Python(AsyncFunction)
            | DefinitionType::Python(DecoratedFunction)
            | DefinitionType::Python(DecoratedAsyncFunction) => {
                Some(DefinitionType::Python(Function))
            }
            DefinitionType::Python(Lambda) => Some(DefinitionType::Python(Lambda)),
            _ => None,
        }
    }

    /// Determine the relationship type between parent and child definitions using proper types
    fn get_definition_relationship_type(
        &self,
        parent_type: &DefinitionType,
        child_type: &DefinitionType,
    ) -> Option<String> {
        use PythonDefinitionType::*;

        let parent_type = self.simplify_definition_type(parent_type)?;
        let child_type = self.simplify_definition_type(child_type)?;

        match (parent_type, child_type) {
            (DefinitionType::Python(Class), DefinitionType::Python(Class)) => {
                Some("CLASS_TO_CLASS".to_string())
            }
            (DefinitionType::Python(Class), DefinitionType::Python(Method)) => {
                Some("CLASS_TO_METHOD".to_string())
            }
            (DefinitionType::Python(Class), DefinitionType::Python(Lambda)) => {
                Some("CLASS_TO_LAMBDA".to_string())
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Class)) => {
                Some("METHOD_TO_CLASS".to_string())
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Function)) => {
                Some("METHOD_TO_FUNCTION".to_string())
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Lambda)) => {
                Some("METHOD_TO_LAMBDA".to_string())
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Function)) => {
                Some("FUNCTION_TO_FUNCTION".to_string())
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Class)) => {
                Some("FUNCTION_TO_CLASS".to_string())
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Lambda)) => {
                Some("FUNCTION_TO_LAMBDA".to_string())
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Lambda)) => {
                Some("LAMBDA_TO_LAMBDA".to_string())
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Class)) => {
                Some("LAMBDA_TO_CLASS".to_string())
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Function)) => {
                Some("LAMBDA_TO_FUNCTION".to_string())
            }
            _ => None, // Unknown or unsupported relationship
        }
    }

    /// Calculate approximate line number from byte position
    fn calculate_line_number(&self, definition: &PythonDefinitionInfo) -> i32 {
        // Use the line number from the match info (1-indexed)
        definition.match_info.range.start.line as i32
    }
}
