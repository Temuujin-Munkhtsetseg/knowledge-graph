use std::collections::HashMap;

use parser_core::kotlin::{
    ast::kotlin_fqn_to_string,
    types::{
        KotlinDefinitionInfo, KotlinDefinitionType, KotlinFqn, KotlinFqnPartType,
        KotlinImportedSymbolInfo, KotlinReferenceTarget, KotlinReferenceType,
        KotlinTargetResolution,
    },
};

use crate::{
    analysis::types::{
        DefinitionLocation, DefinitionNode, DefinitionRelationship, DefinitionType,
        FileDefinitionRelationship, FileImportedSymbolRelationship, FqnType, ImportIdentifier,
        ImportType, ImportedSymbolNode,
    },
    parsing::processor::{FileProcessingResult, References},
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
                        location.clone(),
                    );

                    // Only add file definition relationship for top-level definitions
                    if self.is_top_level_definition(&fqn) {
                        file_definition_relationships.push(FileDefinitionRelationship {
                            file_path: relative_file_path.to_string(),
                            definition_fqn: fqn_string.clone(),
                            relationship_type: RelationshipType::FileDefines,
                            definition_location: location.clone(),
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

    /// Process imported symbols from a file result and update the import map
    pub fn process_imports(
        &self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        imported_symbol_map: &mut HashMap<(String, String), Vec<ImportedSymbolNode>>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
    ) {
        if let Some(imported_symbols) = &file_result.imported_symbols
            && let Some(imports) = imported_symbols.iter_kotlin()
        {
            for imported_symbol in imports {
                let identifier = self.create_imported_symbol_identifier(imported_symbol);

                let imported_symbol_node = ImportedSymbolNode::new(
                    ImportType::Kotlin(imported_symbol.import_type),
                    imported_symbol.import_path.clone(),
                    identifier,
                );

                imported_symbol_map.insert(
                    (imported_symbol.import_path.clone(), "".to_string()),
                    vec![imported_symbol_node.clone()],
                );

                file_import_relationships.push(FileImportedSymbolRelationship {
                    file_path: relative_file_path.to_string(),
                    imported_symbol: imported_symbol_node,
                    relationship_type: RelationshipType::FileImports,
                });
            }
        }
    }

    pub fn process_references(
        &self,
        file_references: &Option<References>,
        relative_file_path: &str,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        let file_path = relative_file_path.to_string();
        if let Some(references) = file_references
            && let Some(references) = references.iter_kotlin()
        {
            for reference in references {
                let source_definition_fqn = reference.scope.as_ref().map(kotlin_fqn_to_string);
                if source_definition_fqn.is_none() {
                    continue;
                }

                let source_definition =
                    definition_map.get(&(source_definition_fqn.unwrap(), file_path.clone()));
                if source_definition.is_none() {
                    continue;
                }

                let source_definition = source_definition.unwrap();
                match &reference.target {
                    KotlinReferenceTarget::Resolved(KotlinTargetResolution::Definition(
                        target_definition_info,
                    )) => {
                        let target_definition = definition_map.get(&(
                            kotlin_fqn_to_string(&target_definition_info.fqn),
                            relative_file_path.to_string(),
                        ));
                        if target_definition.is_none() {
                            continue;
                        }

                        let relationship = self.create_definition_relationship(
                            &source_definition.0,
                            &target_definition.unwrap().0,
                            match reference.reference_type {
                                KotlinReferenceType::MethodCall => RelationshipType::Calls,
                                KotlinReferenceType::PropertyReference => {
                                    RelationshipType::PropertyReference
                                }
                            },
                        );

                        definition_relationships.push(relationship);
                    }
                    KotlinReferenceTarget::Resolved(KotlinTargetResolution::PartialResolution(
                        expression,
                    )) => {
                        for part in &expression.parts {
                            match &part.target {
                                Some(KotlinReferenceTarget::Resolved(
                                    KotlinTargetResolution::Definition(target_definition_info),
                                )) => {
                                    let target_definition = definition_map.get(&(
                                        kotlin_fqn_to_string(&target_definition_info.fqn),
                                        relative_file_path.to_string(),
                                    ));
                                    if target_definition.is_none() {
                                        continue;
                                    }

                                    let relationship = self.create_definition_relationship(
                                        &source_definition.0,
                                        &target_definition.unwrap().0,
                                        match reference.reference_type {
                                            KotlinReferenceType::MethodCall => {
                                                RelationshipType::Calls
                                            }
                                            KotlinReferenceType::PropertyReference => {
                                                RelationshipType::PropertyReference
                                            }
                                        },
                                    );

                                    definition_relationships.push(relationship);
                                }
                                _ => break,
                            }
                        }
                    }
                    _ => continue,
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
            if let Some(parent_fqn) = self.get_parent_fqn_string(child_fqn)
                && let Some((parent_def, _)) =
                    definition_map.get(&(parent_fqn.clone(), child_file_path.to_string()))
                && let Some(relationship_type) = self.get_definition_relationship_type(
                    &parent_def.definition_type,
                    &child_def.definition_type,
                )
            {
                definition_relationships.push(DefinitionRelationship {
                    from_file_path: parent_def.location.file_path.clone(),
                    to_file_path: child_def.location.file_path.clone(),
                    from_definition_fqn: parent_fqn,
                    to_definition_fqn: child_fqn_string.clone(),
                    from_location: parent_def.location.clone(),
                    to_location: child_def.location.clone(),
                    relationship_type,
                });
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

    fn create_definition_relationship(
        &self,
        from_definition: &DefinitionNode,
        to_definition: &DefinitionNode,
        relationship_type: RelationshipType,
    ) -> DefinitionRelationship {
        DefinitionRelationship {
            from_file_path: from_definition.location.file_path.clone(),
            to_file_path: to_definition.location.file_path.clone(),
            from_definition_fqn: from_definition.fqn.clone(),
            to_definition_fqn: to_definition.fqn.clone(),
            from_location: from_definition.location.clone(),
            to_location: to_definition.location.clone(),
            relationship_type,
        }
    }

    fn create_definition_location(
        &self,
        definition: &KotlinDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, KotlinFqn)>, String> {
        let location = DefinitionLocation {
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

    fn is_top_level_definition(&self, fqn: &KotlinFqn) -> bool {
        fqn.len() == 1 || (fqn.len() == 2 && fqn[0].node_type == KotlinFqnPartType::Package)
    }

    fn create_imported_symbol_identifier(
        &self,
        imported_symbol: &KotlinImportedSymbolInfo,
    ) -> Option<ImportIdentifier> {
        if imported_symbol.identifier.is_some() {
            return Some(ImportIdentifier {
                name: imported_symbol.identifier.as_ref().unwrap().name.clone(),
                alias: imported_symbol.identifier.as_ref().unwrap().alias.clone(),
            });
        }

        None
    }
}
