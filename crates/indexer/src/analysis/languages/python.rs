use crate::analysis::types::{
    DefinitionImportedSymbolRelationship, DefinitionLocation, DefinitionNode,
    DefinitionRelationship, DefinitionType, FileDefinitionRelationship,
    FileImportedSymbolRelationship, FqnType, ImportIdentifier, ImportType, ImportedSymbolNode,
};
use crate::parsing::processor::{FileProcessingResult, References};
use database::graph::RelationshipType;
use parser_core::python::{
    fqn::python_fqn_to_string,
    types::{
        PythonDefinitionInfo, PythonDefinitionType, PythonFqn, PythonImportedSymbolInfo,
        PythonTargetResolution,
    },
};
use parser_core::references::ReferenceTarget;
use std::collections::HashMap;

// Handles Python-specific analysis operations
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
        if let Some(defs) = file_result.definitions.iter_python() {
            for definition in defs {
                if let Ok(Some((location, fqn))) =
                    self.create_definition_location(definition, relative_file_path)
                {
                    let fqn_string = python_fqn_to_string(&fqn);
                    let definition_node = DefinitionNode::new(
                        fqn_string.clone(),
                        definition.name.clone(),
                        DefinitionType::Python(definition.definition_type),
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
                        (definition_node, FqnType::Python(fqn)),
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
            && let Some(imports) = imported_symbols.iter_python()
        {
            for imported_symbol in imports {
                let scope_fqn_string = if let Some(ref scope) = imported_symbol.scope {
                    python_fqn_to_string(scope)
                } else {
                    "".to_string()
                };

                let imported_symbol_node = self.create_imported_symbol_node(imported_symbol);
                if let Some(imported_symbol_nodes) = imported_symbol_map
                    .get_mut(&(scope_fqn_string.clone(), relative_file_path.to_string()))
                {
                    imported_symbol_nodes.push(imported_symbol_node.clone());
                } else {
                    imported_symbol_map.insert(
                        (scope_fqn_string, relative_file_path.to_string()),
                        vec![imported_symbol_node.clone()],
                    );
                }

                file_import_relationships.push(FileImportedSymbolRelationship {
                    file_path: relative_file_path.to_string(),
                    imported_symbol: imported_symbol_node,
                    relationship_type: RelationshipType::FileImports,
                });
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn process_references(
        &self,
        file_references: &Option<References>,
        relative_file_path: &str,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
    ) {
        let file_path = relative_file_path.to_string();
        if let Some(references) = file_references
            && let Some(references) = references.iter_python()
        {
            for reference in references {
                let source_definition = if let Some(scope) = reference.scope.as_ref() {
                    let fqn_string = python_fqn_to_string(scope);
                    definition_map
                        .get(&(fqn_string, file_path.clone()))
                        .map(|map_value| map_value.0.clone())
                } else {
                    None
                };
                match &reference.target {
                    ReferenceTarget::Resolved(resolved_target) => {
                        match resolved_target {
                            PythonTargetResolution::Definition(target_def_info) => {
                                self.add_definition_reference_relationship(
                                    &file_path,
                                    &source_definition,
                                    target_def_info,
                                    definition_map,
                                    definition_relationships,
                                    file_definition_relationships,
                                    false,
                                );
                            }
                            PythonTargetResolution::ImportedSymbol(target_import_info) => {
                                self.add_imported_symbol_reference_relationship(
                                    &file_path,
                                    &source_definition,
                                    target_import_info,
                                    definition_imported_symbol_relationships,
                                    file_import_relationships,
                                    false,
                                );
                            }
                            PythonTargetResolution::PartialResolution(_symbol_chain) => {
                                // Ignoring until we do phase 3
                                continue;
                            }
                        }
                    }
                    ReferenceTarget::Ambiguous(possible_targets) => {
                        for possible_target in possible_targets {
                            match possible_target {
                                PythonTargetResolution::Definition(target_def_info) => {
                                    self.add_definition_reference_relationship(
                                        &file_path,
                                        &source_definition,
                                        target_def_info,
                                        definition_map,
                                        definition_relationships,
                                        file_definition_relationships,
                                        true,
                                    );
                                }
                                PythonTargetResolution::ImportedSymbol(target_import_info) => {
                                    self.add_imported_symbol_reference_relationship(
                                        &file_path,
                                        &source_definition,
                                        target_import_info,
                                        definition_imported_symbol_relationships,
                                        file_import_relationships,
                                        true,
                                    );
                                }
                                PythonTargetResolution::PartialResolution(_symbol_chain) => {
                                    // Ignoring until we do phase 3
                                    continue;
                                }
                            }
                        }
                    }
                    ReferenceTarget::Unresolved() => {
                        // Ignoring until we do phase 3
                        continue;
                    }
                }
            }
        }
    }

    /// Create definition-to-definition and definition-to-imported-symbol relationships using definitions map
    pub fn add_definition_relationships(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        imported_symbol_map: &HashMap<(String, String), Vec<ImportedSymbolNode>>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
    ) {
        for ((child_fqn_string, child_file_path), (child_def, child_fqn)) in definition_map {
            // Handle definition-to-imported-symbol relationships
            if let Some(imported_symbol_nodes) =
                imported_symbol_map.get(&(child_fqn_string.clone(), child_file_path.clone()))
            {
                for imported_symbol in imported_symbol_nodes {
                    definition_imported_symbol_relationships.push(
                        DefinitionImportedSymbolRelationship {
                            file_path: child_file_path.clone(),
                            definition_fqn: child_fqn_string.clone(),
                            imported_symbol: imported_symbol.clone(),
                            relationship_type: RelationshipType::DefinesImportedSymbol,
                            definition_location: child_def.location.clone(),
                        },
                    );
                }
            }

            // Handle definition-to-definition relationships
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

    #[allow(clippy::too_many_arguments)]
    fn add_definition_reference_relationship(
        &self,
        file_path: &str,
        source_definition: &Option<DefinitionNode>,
        target_definition_info: &PythonDefinitionInfo,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
        is_ambiguous: bool,
    ) {
        let target_definition = definition_map.get(&(
            python_fqn_to_string(&target_definition_info.fqn),
            file_path.to_string(),
        ));

        if target_definition.is_none() {
            return;
        }

        let target_definition = target_definition.unwrap();
        if source_definition.is_none() {
            let relationship = FileDefinitionRelationship {
                file_path: file_path.to_string(),
                definition_fqn: target_definition.0.fqn.clone(),
                relationship_type: if is_ambiguous {
                    RelationshipType::AmbiguouslyCalls
                } else {
                    RelationshipType::Calls
                },
                definition_location: target_definition.0.location.clone(),
            };
            file_definition_relationships.push(relationship);
        } else {
            let source_definition = source_definition.as_ref().unwrap();
            let relationship = DefinitionRelationship {
                from_file_path: source_definition.location.file_path.clone(),
                to_file_path: target_definition.0.location.file_path.clone(),
                from_definition_fqn: source_definition.fqn.clone(),
                to_definition_fqn: target_definition.0.fqn.clone(),
                from_location: source_definition.location.clone(),
                to_location: target_definition.0.location.clone(),
                relationship_type: if is_ambiguous {
                    RelationshipType::AmbiguouslyCalls
                } else {
                    RelationshipType::Calls
                },
            };
            definition_relationships.push(relationship);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn add_imported_symbol_reference_relationship(
        &self,
        file_path: &str,
        source_definition: &Option<DefinitionNode>,
        target_imported_symbol_info: &PythonImportedSymbolInfo,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
        file_import_relationships: &mut Vec<FileImportedSymbolRelationship>,
        is_ambiguous: bool,
    ) {
        let target_imported_symbol = self.create_imported_symbol_node(target_imported_symbol_info);

        if source_definition.is_none() {
            let relationship = FileImportedSymbolRelationship {
                file_path: file_path.to_string(),
                imported_symbol: target_imported_symbol.clone(),
                relationship_type: if is_ambiguous {
                    RelationshipType::AmbiguouslyCalls
                } else {
                    RelationshipType::Calls
                },
            };
            file_import_relationships.push(relationship);
        } else {
            let source_definition = source_definition.as_ref().unwrap();
            let relationship = DefinitionImportedSymbolRelationship {
                file_path: file_path.to_string(),
                definition_fqn: source_definition.fqn.clone(),
                imported_symbol: target_imported_symbol.clone(),
                relationship_type: if is_ambiguous {
                    RelationshipType::AmbiguouslyCalls
                } else {
                    RelationshipType::Calls
                },
                definition_location: source_definition.location.clone(),
            };
            definition_imported_symbol_relationships.push(relationship);
        }
    }

    /// Create a definition location from a definition info
    fn create_definition_location(
        &self,
        definition: &PythonDefinitionInfo,
        file_path: &str,
    ) -> Result<Option<(DefinitionLocation, PythonFqn)>, String> {
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

    fn create_imported_symbol_node(
        &self,
        imported_symbol: &PythonImportedSymbolInfo,
    ) -> ImportedSymbolNode {
        ImportedSymbolNode::new(
            ImportType::Python(imported_symbol.import_type),
            imported_symbol.import_path.clone(),
            self.create_imported_symbol_identifier(imported_symbol),
        )
    }

    fn create_imported_symbol_identifier(
        &self,
        imported_symbol: &PythonImportedSymbolInfo,
    ) -> Option<ImportIdentifier> {
        if imported_symbol.identifier.is_some() {
            return Some(ImportIdentifier {
                name: imported_symbol.identifier.as_ref().unwrap().name.clone(),
                alias: imported_symbol.identifier.as_ref().unwrap().alias.clone(),
            });
        }

        None
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
    ) -> Option<RelationshipType> {
        use PythonDefinitionType::*;

        let parent_type = self.simplify_definition_type(parent_type)?;
        let child_type = self.simplify_definition_type(child_type)?;

        match (parent_type, child_type) {
            (DefinitionType::Python(Class), DefinitionType::Python(Class)) => {
                Some(RelationshipType::ClassToClass)
            }
            (DefinitionType::Python(Class), DefinitionType::Python(Method)) => {
                Some(RelationshipType::ClassToMethod)
            }
            (DefinitionType::Python(Class), DefinitionType::Python(Lambda)) => {
                Some(RelationshipType::ClassToLambda)
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Class)) => {
                Some(RelationshipType::MethodToClass)
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Function)) => {
                Some(RelationshipType::MethodToFunction)
            }
            (DefinitionType::Python(Method), DefinitionType::Python(Lambda)) => {
                Some(RelationshipType::MethodToLambda)
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Function)) => {
                Some(RelationshipType::FunctionToFunction)
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Class)) => {
                Some(RelationshipType::FunctionToClass)
            }
            (DefinitionType::Python(Function), DefinitionType::Python(Lambda)) => {
                Some(RelationshipType::FunctionToLambda)
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Lambda)) => {
                Some(RelationshipType::LambdaToLambda)
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Class)) => {
                Some(RelationshipType::LambdaToClass)
            }
            (DefinitionType::Python(Lambda), DefinitionType::Python(Function)) => {
                Some(RelationshipType::LambdaToFunction)
            }
            _ => None, // Unknown or unsupported relationship
        }
    }

    fn is_top_level_definition(&self, fqn: &PythonFqn) -> bool {
        fqn.len() == 1
    }
}
