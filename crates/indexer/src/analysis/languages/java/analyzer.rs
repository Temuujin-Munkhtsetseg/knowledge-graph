use std::collections::HashMap;

use database::graph::RelationshipType;
use parser_core::java::{
    ast::java_fqn_to_string,
    types::{
        JavaDefinitionInfo, JavaDefinitionType, JavaFqn, JavaFqnPartType, JavaImportedSymbolInfo,
    },
};

use crate::{
    analysis::{
        languages::java::expression_resolver::ExpressionResolver,
        types::{
            DefinitionLocation, DefinitionNode, DefinitionRelationship, DefinitionType,
            FileDefinitionRelationship, FileImportedSymbolRelationship, FqnType, ImportIdentifier,
            ImportType, ImportedSymbolLocation, ImportedSymbolNode,
        },
    },
    parsing::processor::{FileProcessingResult, References},
};

#[derive(Default)]
pub struct JavaAnalyzer {
    expression_resolver: ExpressionResolver,
}

impl JavaAnalyzer {
    pub fn new() -> Self {
        Self {
            expression_resolver: ExpressionResolver::new(),
        }
    }

    pub fn process_definitions(
        &mut self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
    ) {
        if let Some(defs) = file_result.definitions.iter_java() {
            for definition in defs {
                if matches!(definition.definition_type, JavaDefinitionType::Package) {
                    self.expression_resolver
                        .add_file(definition.name.clone(), relative_file_path.to_string());
                    continue;
                }

                if let Ok(Some((location, fqn))) =
                    self.create_definition_location(definition, relative_file_path)
                {
                    let fqn_string = java_fqn_to_string(&fqn);
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.clone(),
                        DefinitionType::Java(definition.definition_type),
                        location.clone(),
                    );

                    self.expression_resolver.add_definition(
                        relative_file_path.to_string(),
                        definition.clone(),
                        definition_node.clone(),
                    );

                    // We don't want to index local variables, parameters, or fields
                    if definition.definition_type == JavaDefinitionType::LocalVariable
                        || definition.definition_type == JavaDefinitionType::Parameter
                        || definition.definition_type == JavaDefinitionType::Field
                    {
                        continue;
                    }

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
                        (definition_node, FqnType::Java(fqn)),
                    );
                }
            }
        }
    }

    /// Process imported symbols from a file result and update the import map
    pub fn process_imports(
        &mut self,
        file_result: &FileProcessingResult,
        relative_file_path: &str,
        imported_symbol_map: &mut HashMap<(String, String), Vec<ImportedSymbolNode>>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
    ) {
        if let Some(imported_symbols) = &file_result.imported_symbols
            && let Some(imports) = imported_symbols.iter_java()
        {
            for imported_symbol in imports {
                let location =
                    self.create_imported_symbol_location(imported_symbol, relative_file_path);
                let identifier = self.create_imported_symbol_identifier(imported_symbol);

                let imported_symbol_node = ImportedSymbolNode::new(
                    ImportType::Java(imported_symbol.import_type),
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

                self.expression_resolver
                    .add_import(relative_file_path.to_string(), imported_symbol);
            }
        }
    }

    /// Process Java references (calls and creations) and create definition relationships
    pub fn process_references(
        &mut self,
        references: &References,
        file_path: &str,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        self.expression_resolver.resolve_references(
            file_path,
            references,
            definition_relationships,
        );
    }

    /// Create definition-to-definition relationships using definitions map
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
            DefinitionType::Java(Constructor) => Some(DefinitionType::Java(Method)),
            DefinitionType::Java(Lambda) => Some(DefinitionType::Java(Lambda)),
            _ => None,
        }
    }

    fn create_definition_location(
        &self,
        definition: &JavaDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, JavaFqn)>, String> {
        // All definitions now have mandatory FQNs
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

    fn is_top_level_definition(&self, fqn: &JavaFqn) -> bool {
        fqn.len() == 1 || (fqn.len() == 2 && fqn[0].node_type == JavaFqnPartType::Package)
    }

    /// Create an imported symbol location from an imported symbol info
    fn create_imported_symbol_location(
        &self,
        imported_symbol: &JavaImportedSymbolInfo,
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
        imported_symbol: &JavaImportedSymbolInfo,
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
