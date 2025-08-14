use std::collections::HashMap;

use database::graph::RelationshipType;
use parser_core::{
    csharp::types::{
        CSharpDefinitionInfo, CSharpDefinitionType, CSharpFqn, CSharpFqnPartType, CSharpImportType,
    },
    imports::ImportedSymbolInfo,
};

use crate::{
    analysis::types::{
        DefinitionLocation, DefinitionNode, DefinitionRelationship, DefinitionType,
        FileDefinitionRelationship, FileImportedSymbolRelationship, FqnType, ImportIdentifier,
        ImportType, ImportedSymbolLocation, ImportedSymbolNode,
    },
    parsing::processor::FileProcessingResult,
};

#[derive(Default)]
pub struct CSharpAnalyzer;

impl CSharpAnalyzer {
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
        if let Some(defs) = file_result.definitions.iter_csharp() {
            for definition in defs {
                if let Ok(Some((location, fqn))) =
                    self.create_definition_location(definition, relative_file_path)
                {
                    let fqn_string = self.csharp_fqn_to_string(&fqn);
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.clone(),
                        DefinitionType::CSharp(definition.definition_type),
                        location.clone(),
                    );

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
                        (definition_node, FqnType::CSharp(fqn)),
                    );
                }
            }
        }
    }

    pub fn process_imports(
        &self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        imported_symbol_map: &mut HashMap<(String, String), Vec<ImportedSymbolNode>>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
    ) {
        if let Some(imported_symbols) = &file_result.imported_symbols
            && let Some(imports) = imported_symbols.iter_csharp()
        {
            for imported_symbol in imports {
                let location =
                    self.create_imported_symbol_location(imported_symbol, relative_file_path);
                let identifier = self.create_imported_symbol_identifier(imported_symbol);

                let imported_symbol_node = ImportedSymbolNode::new(
                    ImportType::CSharp(imported_symbol.import_type),
                    imported_symbol.import_path.clone(),
                    identifier,
                    location.clone(),
                );

                imported_symbol_map.insert(
                    (
                        imported_symbol.import_path.clone(),
                        relative_file_path.to_string(),
                    ),
                    vec![imported_symbol_node],
                );

                file_import_relationships.push(FileImportedSymbolRelationship {
                    file_path: relative_file_path.to_string(),
                    import_location: location.clone(),
                    relationship_type: RelationshipType::FileImports,
                });
            }
        }
    }

    pub fn add_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        for ((child_fqn_string, child_file_path), (child_def, child_fqn)) in definition_map {
            if let Some(parent_fqn_string) = self.get_parent_fqn_string(child_fqn)
                && let Some((parent_def, _)) =
                    definition_map.get(&(parent_fqn_string.clone(), child_file_path.to_string()))
                && let Some(relationship_type) = self.get_definition_relationship_type(
                    &parent_def.definition_type,
                    &child_def.definition_type,
                )
            {
                definition_relationships.push(DefinitionRelationship {
                    from_file_path: parent_def.location.file_path.clone(),
                    to_file_path: child_def.location.file_path.clone(),
                    from_definition_fqn: parent_fqn_string,
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
            FqnType::CSharp(csharp_fqn) => {
                if csharp_fqn.len() <= 1 {
                    return None;
                }

                let parent_parts: Vec<String> = csharp_fqn[..csharp_fqn.len() - 1]
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
        let parent_type = self.simplify_definition_type(parent_type)?;
        let child_type = self.simplify_definition_type(child_type)?;

        match (parent_type, child_type) {
            (
                DefinitionType::CSharp(CSharpDefinitionType::Class),
                DefinitionType::CSharp(CSharpDefinitionType::Class),
            ) => Some(RelationshipType::ClassToClass),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Class),
                DefinitionType::CSharp(CSharpDefinitionType::Interface),
            ) => Some(RelationshipType::ClassToInterface),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Class),
                DefinitionType::CSharp(CSharpDefinitionType::InstanceMethod),
            ) => Some(RelationshipType::ClassToMethod),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Class),
                DefinitionType::CSharp(CSharpDefinitionType::StaticMethod),
            ) => Some(RelationshipType::ClassToMethod),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Class),
                DefinitionType::CSharp(CSharpDefinitionType::Property),
            ) => Some(RelationshipType::ClassToProperty),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Class),
                DefinitionType::CSharp(CSharpDefinitionType::Constructor),
            ) => Some(RelationshipType::ClassToConstructor),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Class),
                DefinitionType::CSharp(CSharpDefinitionType::Enum),
            ) => Some(RelationshipType::ClassToClass),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Class),
                DefinitionType::CSharp(CSharpDefinitionType::Lambda),
            ) => Some(RelationshipType::ClassToLambda),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Interface),
                DefinitionType::CSharp(CSharpDefinitionType::Interface),
            ) => Some(RelationshipType::InterfaceToInterface),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Interface),
                DefinitionType::CSharp(CSharpDefinitionType::Class),
            ) => Some(RelationshipType::InterfaceToClass),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Interface),
                DefinitionType::CSharp(CSharpDefinitionType::InstanceMethod),
            ) => Some(RelationshipType::InterfaceToMethod),
            (
                DefinitionType::CSharp(CSharpDefinitionType::Interface),
                DefinitionType::CSharp(CSharpDefinitionType::Property),
            ) => Some(RelationshipType::InterfaceToProperty),
            (
                DefinitionType::CSharp(CSharpDefinitionType::InstanceMethod),
                DefinitionType::CSharp(CSharpDefinitionType::InstanceMethod),
            ) => Some(RelationshipType::MethodToMethod),
            (
                DefinitionType::CSharp(CSharpDefinitionType::StaticMethod),
                DefinitionType::CSharp(CSharpDefinitionType::StaticMethod),
            ) => Some(RelationshipType::MethodToMethod),
            (
                DefinitionType::CSharp(CSharpDefinitionType::InstanceMethod),
                DefinitionType::CSharp(CSharpDefinitionType::Lambda),
            ) => Some(RelationshipType::MethodToLambda),
            (
                DefinitionType::CSharp(CSharpDefinitionType::StaticMethod),
                DefinitionType::CSharp(CSharpDefinitionType::Lambda),
            ) => Some(RelationshipType::MethodToLambda),
            _ => None,
        }
    }

    fn simplify_definition_type(&self, definition_type: &DefinitionType) -> Option<DefinitionType> {
        match definition_type {
            DefinitionType::CSharp(CSharpDefinitionType::Class) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Class))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Struct) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Class))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Record) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Class))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Enum) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Class))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Interface) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Interface))
            }
            DefinitionType::CSharp(CSharpDefinitionType::InstanceMethod) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::InstanceMethod))
            }
            DefinitionType::CSharp(CSharpDefinitionType::StaticMethod) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::StaticMethod))
            }
            DefinitionType::CSharp(CSharpDefinitionType::ExtensionMethod) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::StaticMethod))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Property) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Property))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Constructor) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Constructor))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Lambda) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Lambda))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Field) => None,
            DefinitionType::CSharp(CSharpDefinitionType::Delegate) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Class))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Finalizer) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::InstanceMethod))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Operator) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::StaticMethod))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Indexer) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Property))
            }
            DefinitionType::CSharp(CSharpDefinitionType::Event) => None,
            DefinitionType::CSharp(CSharpDefinitionType::AnonymousType) => {
                Some(DefinitionType::CSharp(CSharpDefinitionType::Class))
            }
            _ => None,
        }
    }

    fn create_definition_location(
        &self,
        definition: &CSharpDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, CSharpFqn)>, String> {
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

    fn is_top_level_definition(&self, fqn: &CSharpFqn) -> bool {
        fqn.len() == 1 || (fqn.len() == 2 && fqn[0].node_type == CSharpFqnPartType::Namespace)
    }

    fn create_imported_symbol_location(
        &self,
        imported_symbol: &ImportedSymbolInfo<CSharpImportType, CSharpFqn>,
        file_path: &str,
    ) -> ImportedSymbolLocation {
        ImportedSymbolLocation {
            file_path: file_path.to_string(),
            start_byte: imported_symbol.range.byte_offset.0 as i64,
            end_byte: imported_symbol.range.byte_offset.1 as i64,
            start_line: imported_symbol.range.start.line as i32,
            end_line: imported_symbol.range.end.line as i32,
            start_col: imported_symbol.range.start.column as i32,
            end_col: imported_symbol.range.end.column as i32,
        }
    }

    fn create_imported_symbol_identifier(
        &self,
        imported_symbol: &ImportedSymbolInfo<CSharpImportType, CSharpFqn>,
    ) -> Option<ImportIdentifier> {
        if imported_symbol.identifier.is_some() {
            return Some(ImportIdentifier {
                name: imported_symbol.identifier.as_ref().unwrap().name.clone(),
                alias: imported_symbol.identifier.as_ref().unwrap().alias.clone(),
            });
        }

        None
    }

    fn csharp_fqn_to_string(&self, fqn: &CSharpFqn) -> String {
        fqn.iter()
            .map(|part| part.node_name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}
