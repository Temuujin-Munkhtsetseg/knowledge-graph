use std::collections::HashMap;

use parser_core::kotlin::{
    fqn::kotlin_fqn_to_string,
    types::{KotlinDefinitionInfo, KotlinDefinitionType, KotlinFqn, KotlinFqnPartType},
};

use crate::{
    analysis::types::{
        DefinitionLocation, DefinitionNode, DefinitionRelationship, DefinitionType,
        FileDefinitionRelationship, FqnType,
    },
    indexer::FileProcessingResult,
};
use database::graph::RelationshipType;

#[derive(Default)]
pub struct KotlinAnalyzer;

impl KotlinAnalyzer {
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
        if let Some(defs) = file_result.definitions.iter_kotlin() {
            for definition in defs {
                if let Ok(Some((location, fqn))) =
                    self.create_definition_location(definition, relative_file_path)
                {
                    let fqn_string = kotlin_fqn_to_string(&fqn);
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.clone(),
                        DefinitionType::Kotlin(definition.definition_type),
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
                        (definition_node, FqnType::Kotlin(fqn)),
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
            if let Some(parent_fqn) = self.get_parent_fqn_string(child_fqn) {
                if let Some((parent_def, _)) =
                    definition_map.get(&(parent_fqn.clone(), child_file_path.to_string()))
                {
                    if let Some(relationship_type) = self.get_definition_relationship_type(
                        &parent_def.definition_type,
                        &child_def.definition_type,
                    ) {
                        definition_relationships.push(DefinitionRelationship {
                            from_file_path: parent_def.location.file_path.clone(),
                            to_file_path: child_def.location.file_path.clone(),
                            from_definition_fqn: parent_fqn,
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
            FqnType::Kotlin(kotlin_fqn) => {
                if kotlin_fqn.len() <= 1 {
                    return None;
                }

                let parent_parts: Vec<String> = kotlin_fqn[..kotlin_fqn.len() - 1]
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
        use KotlinDefinitionType::*;

        let parent_type = self.simplify_definition_type(parent_type)?;
        let child_type = self.simplify_definition_type(child_type)?;

        match (parent_type, child_type) {
            // Class relationships
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Class)) => {
                Some(RelationshipType::ClassToClass)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Interface)) => {
                Some(RelationshipType::ClassToInterface)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Function)) => {
                Some(RelationshipType::ClassToMethod)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Property)) => {
                Some(RelationshipType::ClassToProperty)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Lambda)) => {
                Some(RelationshipType::ClassToLambda)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(Constructor)) => {
                Some(RelationshipType::ClassToConstructor)
            }
            (DefinitionType::Kotlin(Class), DefinitionType::Kotlin(EnumEntry)) => {
                Some(RelationshipType::ClassToEnumEntry)
            }
            // Interface relationships
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Interface)) => {
                Some(RelationshipType::InterfaceToInterface)
            }
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Class)) => {
                Some(RelationshipType::InterfaceToClass)
            }
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Function)) => {
                Some(RelationshipType::InterfaceToMethod)
            }
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Property)) => {
                Some(RelationshipType::InterfaceToProperty)
            }
            (DefinitionType::Kotlin(Interface), DefinitionType::Kotlin(Lambda)) => {
                Some(RelationshipType::InterfaceToLambda)
            }
            // Function relationships
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Function)) => {
                Some(RelationshipType::MethodToMethod)
            }
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Property)) => {
                Some(RelationshipType::MethodToProperty)
            }
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Lambda)) => {
                Some(RelationshipType::MethodToLambda)
            }
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Class)) => {
                Some(RelationshipType::MethodToClass)
            }
            (DefinitionType::Kotlin(Function), DefinitionType::Kotlin(Interface)) => {
                Some(RelationshipType::MethodToInterface)
            }
            // Lambda relationships
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Lambda)) => {
                Some(RelationshipType::LambdaToLambda)
            }
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Class)) => {
                Some(RelationshipType::LambdaToClass)
            }
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Function)) => {
                Some(RelationshipType::LambdaToMethod)
            }
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Property)) => {
                Some(RelationshipType::LambdaToProperty)
            }
            (DefinitionType::Kotlin(Lambda), DefinitionType::Kotlin(Interface)) => {
                Some(RelationshipType::LambdaToInterface)
            }
            _ => None,
        }
    }

    fn simplify_definition_type(&self, definition_type: &DefinitionType) -> Option<DefinitionType> {
        use KotlinDefinitionType::*;

        match definition_type {
            DefinitionType::Kotlin(Class) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(DataClass) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(ValueClass) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(AnnotationClass) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(Object) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(CompanionObject) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(Enum) => Some(DefinitionType::Kotlin(Class)),
            DefinitionType::Kotlin(Interface) => Some(DefinitionType::Kotlin(Interface)),
            DefinitionType::Kotlin(EnumEntry) => Some(DefinitionType::Kotlin(EnumEntry)),
            DefinitionType::Kotlin(Constructor) => Some(DefinitionType::Kotlin(Constructor)),
            DefinitionType::Kotlin(Function) => Some(DefinitionType::Kotlin(Function)),
            DefinitionType::Kotlin(Property) => Some(DefinitionType::Kotlin(Property)),
            DefinitionType::Kotlin(Lambda) => Some(DefinitionType::Kotlin(Lambda)),
            _ => None,
        }
    }

    fn create_definition_location(
        &self,
        definition: &KotlinDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, KotlinFqn)>, String> {
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
                "Skipping definition '{}' without FQN in file '{}'.",
                definition.name,
                file_path
            );

            Ok(None)
        }
    }

    fn is_top_level_definition(&self, fqn: &KotlinFqn) -> bool {
        fqn.len() == 1 || (fqn.len() == 2 && fqn[0].node_type == KotlinFqnPartType::Package)
    }
}
