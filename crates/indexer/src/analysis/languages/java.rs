use std::collections::HashMap;

use database::graph::RelationshipType;
use parser_core::java::{
    fqn::java_fqn_to_string,
    types::{JavaDefinitionInfo, JavaDefinitionType, JavaFqn, JavaFqnPartType},
};

use crate::{
    analysis::types::{
        DefinitionLocation, DefinitionNode, DefinitionRelationship, DefinitionType,
        FileDefinitionRelationship, FqnType,
    },
    indexer::FileProcessingResult,
};

#[derive(Default)]
pub struct JavaAnalyzer;

impl JavaAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn process_definitions(
        &self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
    ) {
        if let Some(defs) = file_result.definitions.iter_java() {
            for definition in defs {
                if let Ok(Some((location, fqn))) =
                    self.create_definition_location(definition, relative_file_path)
                {
                    let fqn_string = java_fqn_to_string(&fqn);
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.clone(),
                        DefinitionType::Java(definition.definition_type),
                        location,
                    );

                    // Only add file definition relationship for top-level definitions
                    if self.is_top_level_definition(&fqn) {
                        file_definition_relationships.push(FileDefinitionRelationship {
                            file_path: relative_file_path.to_string(),
                            definition_fqn: fqn_string.clone(),
                            relationship_type: RelationshipType::FileDefines,
                        });
                    }

                    definition_map.insert(
                        (fqn_string.clone(), relative_file_path.to_string()),
                        (definition_node, FqnType::Java(fqn)),
                    );
                }
            }
        }
    }

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

    fn get_parent_fqn_string(&self, fqn: &FqnType) -> Option<String> {
        match fqn {
            FqnType::Java(java_fqn) => {
                if java_fqn.len() <= 1 {
                    return None;
                }

                let parent_parts: Vec<String> = java_fqn[..java_fqn.len() - 1]
                    .iter()
                    .map(|part| part.node_name.clone())
                    .collect();

                if parent_parts.is_empty() {
                    None
                } else {
                    Some(parent_parts.join("."))
                }
            }
            _ => None,
        }
    }

    fn get_definition_relationship_type(
        &self,
        parent_type: &DefinitionType,
        child_type: &DefinitionType,
    ) -> Option<RelationshipType> {
        use JavaDefinitionType::*;

        let parent_type = self.simplify_definition_type(parent_type)?;
        let child_type = self.simplify_definition_type(child_type)?;

        match (parent_type, child_type) {
            // Class relationships
            (DefinitionType::Java(Class), DefinitionType::Java(Class)) => {
                Some(RelationshipType::ClassToClass)
            }
            (DefinitionType::Java(Class), DefinitionType::Java(Constructor)) => {
                Some(RelationshipType::ClassToConstructor)
            }
            (DefinitionType::Java(Class), DefinitionType::Java(Interface)) => {
                Some(RelationshipType::ClassToInterface)
            }
            (DefinitionType::Java(Class), DefinitionType::Java(EnumConstant)) => {
                Some(RelationshipType::ClassToEnumEntry)
            }
            (DefinitionType::Java(Class), DefinitionType::Java(Method)) => {
                Some(RelationshipType::ClassToMethod)
            }
            (DefinitionType::Java(Class), DefinitionType::Java(Lambda)) => {
                Some(RelationshipType::ClassToLambda)
            }
            // Interface relationships
            (DefinitionType::Java(Interface), DefinitionType::Java(Interface)) => {
                Some(RelationshipType::InterfaceToInterface)
            }
            (DefinitionType::Java(Interface), DefinitionType::Java(Class)) => {
                Some(RelationshipType::InterfaceToClass)
            }
            (DefinitionType::Java(Interface), DefinitionType::Java(Method)) => {
                Some(RelationshipType::InterfaceToMethod)
            }
            (DefinitionType::Java(Interface), DefinitionType::Java(Lambda)) => {
                Some(RelationshipType::InterfaceToLambda)
            }
            // Method relationships
            (DefinitionType::Java(Method), DefinitionType::Java(Method)) => {
                Some(RelationshipType::MethodToMethod)
            }
            (DefinitionType::Java(Method), DefinitionType::Java(Class)) => {
                Some(RelationshipType::MethodToClass)
            }
            (DefinitionType::Java(Method), DefinitionType::Java(Interface)) => {
                Some(RelationshipType::MethodToInterface)
            }
            (DefinitionType::Java(Method), DefinitionType::Java(Lambda)) => {
                Some(RelationshipType::MethodToLambda)
            }
            // Lambda relationships
            (DefinitionType::Java(Lambda), DefinitionType::Java(Lambda)) => {
                Some(RelationshipType::LambdaToLambda)
            }
            (DefinitionType::Java(Lambda), DefinitionType::Java(Class)) => {
                Some(RelationshipType::LambdaToClass)
            }
            (DefinitionType::Java(Lambda), DefinitionType::Java(Method)) => {
                Some(RelationshipType::LambdaToMethod)
            }
            (DefinitionType::Java(Lambda), DefinitionType::Java(Interface)) => {
                Some(RelationshipType::LambdaToInterface)
            }
            _ => None,
        }
    }

    fn simplify_definition_type(&self, definition_type: &DefinitionType) -> Option<DefinitionType> {
        use JavaDefinitionType::*;

        match definition_type {
            DefinitionType::Java(Class) => Some(DefinitionType::Java(Class)),
            DefinitionType::Java(Enum) => Some(DefinitionType::Java(Class)),
            DefinitionType::Java(AnnotationDeclaration) => Some(DefinitionType::Java(Class)),
            DefinitionType::Java(Record) => Some(DefinitionType::Java(Class)),
            DefinitionType::Java(Interface) => Some(DefinitionType::Java(Interface)),
            DefinitionType::Java(EnumConstant) => Some(DefinitionType::Java(EnumConstant)),
            DefinitionType::Java(Method) => Some(DefinitionType::Java(Method)),
            DefinitionType::Java(Constructor) => Some(DefinitionType::Java(Constructor)),
            DefinitionType::Java(Lambda) => Some(DefinitionType::Java(Lambda)),
            _ => None,
        }
    }

    fn create_definition_location(
        &self,
        definition: &JavaDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, JavaFqn)>, String> {
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
                "Skipping definition '{}' without FQN in file '{}'.",
                definition.name,
                file_path
            );

            Ok(None)
        }
    }

    fn calculate_line_number(&self, definition: &JavaDefinitionInfo) -> i32 {
        definition.match_info.range.start.line as i32
    }

    fn is_top_level_definition(&self, fqn: &JavaFqn) -> bool {
        fqn.len() == 1 || (fqn.len() == 2 && fqn[0].node_type == JavaFqnPartType::Package)
    }
}
