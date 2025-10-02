pub mod files;
pub mod languages;
pub mod types;

use crate::analysis::types::{
    DefinitionImportedSymbolRelationship, DefinitionNode, DefinitionRelationship, DirectoryNode,
    DirectoryRelationship, FileDefinitionRelationship, FileImportedSymbolRelationship, FileNode,
    FqnType, GraphData, ImportedSymbolDefinitionRelationship, ImportedSymbolFileRelationship,
    ImportedSymbolImportedSymbolRelationship, ImportedSymbolLocation, ImportedSymbolNode,
    OptimizedFileTree,
};
use crate::parsing::processor::{FileProcessingResult, References};
use database::graph::RelationshipType;
use parser_core::parser::SupportedLanguage;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

// Re-export the sub-module functionality
pub use files::FileSystemAnalyzer;
pub use languages::csharp::CSharpAnalyzer;
pub use languages::java::JavaAnalyzer;
pub use languages::kotlin::KotlinAnalyzer;
pub use languages::python::PythonAnalyzer;
pub use languages::ruby::RubyAnalyzer;
pub use languages::rust::RustAnalyzer;
pub use languages::typescript::TypeScriptAnalyzer;

/// Analysis service that orchestrates the transformation of parsing results into graph data
pub struct AnalysisService {
    repository_name: String,
    repository_path: String,
    filesystem_analyzer: FileSystemAnalyzer,
    ruby_analyzer: RubyAnalyzer,
    python_analyzer: PythonAnalyzer,
    kotlin_analyzer: KotlinAnalyzer,
    java_analyzer: JavaAnalyzer,
    csharp_analyzer: CSharpAnalyzer,
    typescript_analyzer: TypeScriptAnalyzer,
    rust_analyzer: RustAnalyzer,
}

impl AnalysisService {
    /// Create a new analysis service
    pub fn new(repository_name: String, repository_path: String) -> Self {
        let filesystem_analyzer =
            FileSystemAnalyzer::new(repository_name.clone(), repository_path.clone());
        let ruby_analyzer = RubyAnalyzer::new();
        let python_analyzer = PythonAnalyzer::new();
        let kotlin_analyzer = KotlinAnalyzer::new();
        let java_analyzer = JavaAnalyzer::new();
        let csharp_analyzer = CSharpAnalyzer::new();
        let typescript_analyzer = TypeScriptAnalyzer::new();
        let rust_analyzer = RustAnalyzer::new();

        Self {
            repository_name,
            repository_path,
            filesystem_analyzer,
            ruby_analyzer,
            python_analyzer,
            kotlin_analyzer,
            java_analyzer,
            csharp_analyzer,
            typescript_analyzer,
            rust_analyzer,
        }
    }

    /// Analyze file processing results and transform them into graph data
    pub fn analyze_results(
        mut self,
        file_results: Vec<FileProcessingResult>,
    ) -> Result<GraphData, String> {
        let start_time = Instant::now();
        log::info!(
            "Starting analysis of {} file results for repository '{}' at '{}'",
            file_results.len(),
            self.repository_name,
            self.repository_path
        );

        let mut definition_nodes: Vec<DefinitionNode> = Vec::new();
        let mut imported_symbol_nodes: Vec<ImportedSymbolNode> = Vec::new();
        let mut directory_nodes: Vec<DirectoryNode> = Vec::new();
        let mut file_nodes: Vec<FileNode> = Vec::new();
        let mut directory_relationships: Vec<DirectoryRelationship> = Vec::new();
        let mut file_definition_relationships: Vec<FileDefinitionRelationship> = Vec::new();
        let mut file_imported_symbol_relationships: Vec<FileImportedSymbolRelationship> =
            Vec::new();
        let mut definition_relationships: Vec<DefinitionRelationship> = Vec::new();

        let mut definition_imported_symbol_relationships: Vec<
            DefinitionImportedSymbolRelationship,
        > = Vec::new();
        let mut imported_symbol_imported_symbol_relationships: Vec<
            ImportedSymbolImportedSymbolRelationship,
        > = Vec::new();
        let mut imported_symbol_definition_relationships: Vec<
            ImportedSymbolDefinitionRelationship,
        > = Vec::new();
        let mut imported_symbol_file_relationships: Vec<ImportedSymbolFileRelationship> =
            Vec::new();

        // TODO: Deprecate these. Can make directory_nodes and directory_relationships HashMaps.
        let mut created_directories = HashSet::new();
        let mut created_dir_relationships = HashSet::new();

        let results_by_language = self.group_results_by_language(file_results);
        for (language, results) in results_by_language {
            let mut definition_map = HashMap::new(); // (fqn_str, file_path) -> (node, fqn)
            let mut imported_symbol_map = HashMap::new(); // (fqn_str, file_path) -> [node, ...]
            let mut imported_symbol_to_imported_symbols = HashMap::new();
            let mut imported_symbol_to_definitions = HashMap::new();
            let mut imported_symbol_to_files = HashMap::new();

            let mut file_references = Vec::new();
            for file_result in results {
                self.extract_file_system_entities(
                    &file_result,
                    &mut file_nodes,
                    &mut directory_nodes,
                    &mut directory_relationships,
                    &mut created_directories,
                    &mut created_dir_relationships,
                );
                self.extract_language_entities(
                    &file_result,
                    &mut definition_map,
                    &mut imported_symbol_map,
                    &mut file_definition_relationships,
                    &mut file_imported_symbol_relationships,
                );
                file_references.push((
                    self.filesystem_analyzer
                        .get_relative_path(file_result.file_path.as_str()),
                    file_result.references,
                ));
            }

            self.add_nodes(
                &definition_map,
                &imported_symbol_map,
                &mut definition_nodes,
                &mut imported_symbol_nodes,
            );
            self.add_definition_relationships(
                language,
                &definition_map,
                &imported_symbol_map,
                &mut definition_relationships,
                &mut definition_imported_symbol_relationships,
            );
            if language == SupportedLanguage::Python {
                let file_tree =
                    OptimizedFileTree::new(file_references.iter().map(|(path, _)| path));

                self.extract_import_relationships(
                    language,
                    file_tree,
                    &mut definition_map,
                    &mut imported_symbol_map,
                    &mut imported_symbol_to_imported_symbols,
                    &mut imported_symbol_to_definitions,
                    &mut imported_symbol_to_files,
                    &mut imported_symbol_imported_symbol_relationships,
                    &mut imported_symbol_definition_relationships,
                    &mut imported_symbol_file_relationships,
                );
            }
            self.extract_reference_relationships(
                language,
                file_references,
                &definition_map,
                &imported_symbol_map,
                &mut definition_relationships,
                &mut definition_imported_symbol_relationships,
                &mut file_definition_relationships,
                &mut file_imported_symbol_relationships,
                &imported_symbol_to_imported_symbols,
                &imported_symbol_to_definitions,
                &imported_symbol_to_files,
            );
        }

        let analysis_time = start_time.elapsed();
        log::info!(
            "Analysis completed in {:?}: {} directories, {} files, {} definitions ({} total locations), {} imported symbols ({} total locations), {} total relationships",
            analysis_time,
            directory_nodes.len(),
            file_nodes.len(),
            definition_nodes.len(),
            definition_nodes.iter().map(|_d| 1).sum::<usize>(),
            imported_symbol_nodes.len(),
            imported_symbol_nodes.iter().map(|_i| 1).sum::<usize>(),
            directory_relationships.len()
                + file_definition_relationships.len()
                + file_imported_symbol_relationships.len()
                + definition_relationships.len()
                + definition_imported_symbol_relationships.len()
                + imported_symbol_imported_symbol_relationships.len()
                + imported_symbol_definition_relationships.len()
                + imported_symbol_file_relationships.len()
        );

        Ok(GraphData {
            directory_nodes,
            file_nodes,
            definition_nodes,
            imported_symbol_nodes,
            directory_relationships,
            file_imported_symbol_relationships,
            file_definition_relationships,
            definition_relationships,
            definition_imported_symbol_relationships,
            imported_symbol_imported_symbol_relationships,
            imported_symbol_definition_relationships,
            imported_symbol_file_relationships,
        })
    }

    fn group_results_by_language(
        &self,
        file_results: Vec<FileProcessingResult>,
    ) -> HashMap<SupportedLanguage, Vec<FileProcessingResult>> {
        let mut results_by_language = HashMap::new();

        for file_result in file_results {
            results_by_language
                .entry(file_result.language)
                .or_insert_with(Vec::new)
                .push(file_result);
        }
        results_by_language
    }

    fn extract_file_system_entities(
        &self,
        file_result: &FileProcessingResult,
        file_nodes: &mut Vec<FileNode>,
        directory_nodes: &mut Vec<DirectoryNode>,
        directory_relationships: &mut Vec<DirectoryRelationship>,
        created_directories: &mut HashSet<String>,
        created_dir_relationships: &mut HashSet<(String, String)>,
    ) {
        // Create directory nodes and relationships for this file's path
        self.filesystem_analyzer.create_directory_hierarchy(
            &file_result.file_path,
            directory_nodes,
            directory_relationships,
            created_directories,
            created_dir_relationships,
        );

        // Create file node
        let file_node = self.filesystem_analyzer.create_file_node(file_result);

        // Store the relative path before moving file_node
        let relative_file_path = file_node.path.clone();

        // Create directory-to-file relationship using the same relative path as the FileNode
        if let Some(parent_dir) = self
            .filesystem_analyzer
            .get_parent_directory(&file_result.file_path)
        {
            directory_relationships.push(DirectoryRelationship {
                from_path: parent_dir,
                to_path: relative_file_path.clone(),
                relationship_type: RelationshipType::DirContainsFile,
            });
        }
        file_nodes.push(file_node);
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_language_entities(
        &mut self,
        file_result: &FileProcessingResult,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        imported_symbol_map: &mut HashMap<(String, String), Vec<ImportedSymbolNode>>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
        file_imported_symbol_relationships: &mut Vec<FileImportedSymbolRelationship>,
    ) {
        let relative_path = self
            .filesystem_analyzer
            .get_relative_path(&file_result.file_path);
        match file_result.language {
            SupportedLanguage::Ruby => {
                let _ = self.ruby_analyzer.process_definitions(
                    file_result,
                    &relative_path,
                    definition_map,
                    file_definition_relationships,
                );
            }
            SupportedLanguage::Python => {
                self.python_analyzer.process_definitions(
                    file_result,
                    &relative_path,
                    definition_map,
                    file_definition_relationships,
                );
                self.python_analyzer.process_imports(
                    file_result,
                    &relative_path,
                    imported_symbol_map,
                    file_imported_symbol_relationships,
                );
            }
            SupportedLanguage::Kotlin => {
                self.kotlin_analyzer.process_definitions(
                    file_result,
                    &relative_path,
                    definition_map,
                    file_definition_relationships,
                );
                self.kotlin_analyzer.process_imports(
                    file_result,
                    &relative_path,
                    imported_symbol_map,
                    file_imported_symbol_relationships,
                );
            }
            SupportedLanguage::Java => {
                self.java_analyzer.process_definitions(
                    file_result,
                    &relative_path,
                    definition_map,
                    file_definition_relationships,
                );
                self.java_analyzer.process_imports(
                    file_result,
                    &relative_path,
                    imported_symbol_map,
                    file_imported_symbol_relationships,
                );
            }
            SupportedLanguage::CSharp => {
                self.csharp_analyzer.process_definitions(
                    file_result,
                    &relative_path,
                    definition_map,
                    file_definition_relationships,
                );
                self.csharp_analyzer.process_imports(
                    file_result,
                    &relative_path,
                    imported_symbol_map,
                    file_imported_symbol_relationships,
                );
            }
            SupportedLanguage::TypeScript => {
                self.typescript_analyzer.process_definitions(
                    file_result,
                    &relative_path,
                    definition_map,
                    file_definition_relationships,
                );
                self.typescript_analyzer.process_imports(
                    file_result,
                    &relative_path,
                    imported_symbol_map,
                    file_imported_symbol_relationships,
                );
            }
            SupportedLanguage::Rust => {
                self.rust_analyzer.process_definitions(
                    file_result,
                    &relative_path,
                    definition_map,
                    file_definition_relationships,
                );
                self.rust_analyzer.process_imports(
                    file_result,
                    &relative_path,
                    imported_symbol_map,
                    file_imported_symbol_relationships,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_import_relationships(
        &mut self,
        language: SupportedLanguage,
        file_tree: OptimizedFileTree,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        imported_symbol_map: &mut HashMap<(String, String), Vec<ImportedSymbolNode>>,
        imported_symbol_to_imported_symbols: &mut HashMap<
            ImportedSymbolLocation,
            Vec<ImportedSymbolNode>,
        >,
        imported_symbol_to_definitions: &mut HashMap<ImportedSymbolLocation, Vec<DefinitionNode>>,
        imported_symbol_to_files: &mut HashMap<ImportedSymbolLocation, Vec<String>>,
        imported_symbol_imported_symbol_relationships: &mut Vec<
            ImportedSymbolImportedSymbolRelationship,
        >,
        imported_symbol_definition_relationships: &mut Vec<ImportedSymbolDefinitionRelationship>,
        imported_symbol_file_relationships: &mut Vec<ImportedSymbolFileRelationship>,
    ) {
        if language == SupportedLanguage::Python {
            // Maps imported symbols to their sources (e.g. a definition, another imported symbol, etc.)
            self.python_analyzer.resolve_imported_symbols(
                imported_symbol_map,
                definition_map,
                &file_tree,
                imported_symbol_to_imported_symbols,
                imported_symbol_to_definitions,
                imported_symbol_to_files,
            );

            // Create imported symbol -> imported symbol relationships
            for (source_location, target_imported_symbols) in imported_symbol_to_imported_symbols {
                for target_imported_symbol in target_imported_symbols {
                    let relationship = ImportedSymbolImportedSymbolRelationship {
                        source_location: source_location.clone(),
                        target_location: target_imported_symbol.location.clone(),
                        relationship_type: RelationshipType::ImportedSymbolToImportedSymbol,
                    };
                    imported_symbol_imported_symbol_relationships.push(relationship);
                }
            }

            // Create imported symbol -> definition relationships
            for (source_location, target_definitions) in imported_symbol_to_definitions {
                for target_definition in target_definitions {
                    let relationship = ImportedSymbolDefinitionRelationship {
                        source_location: source_location.clone(),
                        target_location: target_definition.location.clone(),
                        relationship_type: RelationshipType::ImportedSymbolToDefinition,
                    };
                    imported_symbol_definition_relationships.push(relationship);
                }
            }

            // Create imported symbol -> file relationships
            for (source_location, target_files) in imported_symbol_to_files {
                for target_file in target_files {
                    let relationship = ImportedSymbolFileRelationship {
                        source_location: source_location.clone(),
                        target_location: target_file.clone(),
                        relationship_type: RelationshipType::ImportedSymbolToFile,
                    };
                    imported_symbol_file_relationships.push(relationship);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_reference_relationships(
        &mut self,
        language: SupportedLanguage,
        file_references: Vec<(String, Option<References>)>,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        imported_symbol_map: &HashMap<(String, String), Vec<ImportedSymbolNode>>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
        file_imported_symbol_relationships: &mut Vec<FileImportedSymbolRelationship>,
        imported_symbol_to_imported_symbols: &HashMap<
            ImportedSymbolLocation,
            Vec<ImportedSymbolNode>,
        >,
        imported_symbol_to_definitions: &HashMap<ImportedSymbolLocation, Vec<DefinitionNode>>,
        imported_symbol_to_files: &HashMap<ImportedSymbolLocation, Vec<String>>,
    ) {
        for (relative_path, references) in file_references {
            match language {
                SupportedLanguage::Python => {
                    self.python_analyzer.process_references(
                        &references,
                        &relative_path,
                        definition_map,
                        imported_symbol_map,
                        definition_relationships,
                        definition_imported_symbol_relationships,
                        file_definition_relationships,
                        file_imported_symbol_relationships,
                        imported_symbol_to_imported_symbols,
                        imported_symbol_to_definitions,
                        imported_symbol_to_files,
                    );
                }
                SupportedLanguage::Ruby | SupportedLanguage::Java | SupportedLanguage::Kotlin => {
                    if let Some(references) = references {
                        if language == SupportedLanguage::Ruby {
                            self.ruby_analyzer.process_references(
                                &references,
                                &relative_path,
                                definition_relationships,
                            );
                        } else if language == SupportedLanguage::Java {
                            self.java_analyzer.process_references(
                                &references,
                                &relative_path,
                                definition_relationships,
                                definition_imported_symbol_relationships,
                            );
                        } else if language == SupportedLanguage::Kotlin {
                            self.kotlin_analyzer.process_references(
                                &references,
                                &relative_path,
                                definition_relationships,
                                definition_imported_symbol_relationships,
                            );
                        }
                    }
                }
                SupportedLanguage::TypeScript => {
                    self.typescript_analyzer.process_references(
                        &references,
                        &relative_path,
                        definition_relationships,
                        file_definition_relationships,
                    );
                }
                _ => {}
            }
        }
    }

    fn add_nodes(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        imported_symbol_map: &HashMap<(String, String), Vec<ImportedSymbolNode>>,
        definition_nodes: &mut Vec<DefinitionNode>,
        imported_symbol_nodes: &mut Vec<ImportedSymbolNode>,
    ) {
        // Add definition nodes
        let unrolled_definitions: Vec<DefinitionNode> = definition_map
            .values()
            .map(|(def_node, _)| def_node.clone())
            .collect();
        definition_nodes.extend(unrolled_definitions);

        // Add imported symbol nodes
        imported_symbol_nodes.extend(
            imported_symbol_map
                .values()
                .flatten()
                .cloned()
                .collect::<Vec<_>>(),
        );
    }

    fn add_definition_relationships(
        &self,
        language: SupportedLanguage,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        imported_symbol_map: &HashMap<(String, String), Vec<ImportedSymbolNode>>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
        definition_imported_symbol_relationships: &mut Vec<DefinitionImportedSymbolRelationship>,
    ) {
        match language {
            SupportedLanguage::Ruby => {
                self.ruby_analyzer
                    .add_definition_relationships(definition_map, definition_relationships);
            }
            SupportedLanguage::Python => {
                self.python_analyzer.add_definition_relationships(
                    definition_map,
                    imported_symbol_map,
                    definition_relationships,
                    definition_imported_symbol_relationships,
                );
            }
            SupportedLanguage::Kotlin => {
                self.kotlin_analyzer
                    .add_definition_relationships(definition_map, definition_relationships);
            }
            SupportedLanguage::Java => {
                self.java_analyzer
                    .add_definition_relationships(definition_map, definition_relationships);
            }
            SupportedLanguage::CSharp => {
                self.csharp_analyzer
                    .add_definition_relationships(definition_map, definition_relationships);
            }
            SupportedLanguage::TypeScript => {
                self.typescript_analyzer.add_definition_relationships(
                    definition_map,
                    imported_symbol_map,
                    definition_relationships,
                    definition_imported_symbol_relationships,
                );
            }
            SupportedLanguage::Rust => {
                self.rust_analyzer.add_definition_relationships(
                    definition_map,
                    imported_symbol_map,
                    definition_relationships,
                    definition_imported_symbol_relationships,
                );
            }
        }
    }
}

/// Analysis statistics
#[derive(Debug, Clone)]
pub struct AnalysisStats {
    pub total_directories_created: usize,
    pub total_files_analyzed: usize,
    pub total_definitions_created: usize,
    pub total_imported_symbols_created: usize,
    pub total_directory_relationships: usize,
    pub total_file_definition_relationships: usize,
    pub total_file_imported_symbol_relationships: usize,
    pub total_definition_relationships: usize,
    pub total_definition_imported_symbol_relationships: usize,
    pub analysis_duration: Duration,
    pub files_by_language: HashMap<String, usize>,
    pub definitions_by_type: HashMap<String, usize>,
    pub imported_symbols_by_type: HashMap<String, usize>,
    pub relationships_by_type: HashMap<RelationshipType, usize>,
}

impl AnalysisStats {
    /// Create analysis statistics from graph data
    pub fn from_graph_data(graph_data: &GraphData, analysis_duration: Duration) -> Self {
        let mut files_by_language = HashMap::new();
        let mut definitions_by_type = HashMap::new();
        let mut imported_symbols_by_type = HashMap::new();
        let mut relationships_by_type = HashMap::new();

        // Count files by language
        for file_node in &graph_data.file_nodes {
            *files_by_language
                .entry(file_node.language.clone())
                .or_insert(0) += 1;
        }

        // Count definitions by type
        for definition_node in &graph_data.definition_nodes {
            *definitions_by_type
                .entry(definition_node.definition_type.as_str().to_string())
                .or_insert(0) += 1;
        }

        // Count imported symbols by type
        for imported_symbol_node in &graph_data.imported_symbol_nodes {
            *imported_symbols_by_type
                .entry(imported_symbol_node.import_type.as_str().to_string())
                .or_insert(0) += 1;
        }

        // Count relationships by type
        for rel in &graph_data.directory_relationships {
            *relationships_by_type
                .entry(rel.relationship_type)
                .or_insert(0) += 1;
        }
        for rel in &graph_data.file_definition_relationships {
            *relationships_by_type
                .entry(rel.relationship_type)
                .or_insert(0) += 1;
        }
        for rel in &graph_data.file_imported_symbol_relationships {
            *relationships_by_type
                .entry(rel.relationship_type)
                .or_insert(0) += 1;
        }
        for rel in &graph_data.definition_relationships {
            *relationships_by_type
                .entry(rel.relationship_type)
                .or_insert(0) += 1;
        }
        for rel in &graph_data.definition_imported_symbol_relationships {
            *relationships_by_type
                .entry(rel.relationship_type)
                .or_insert(0) += 1;
        }

        Self {
            total_directories_created: graph_data.directory_nodes.len(),
            total_files_analyzed: graph_data.file_nodes.len(),
            total_definitions_created: graph_data.definition_nodes.len(),
            total_imported_symbols_created: graph_data.imported_symbol_nodes.len(),
            total_directory_relationships: graph_data.directory_relationships.len(),
            total_file_definition_relationships: graph_data.file_definition_relationships.len(),
            total_file_imported_symbol_relationships: graph_data
                .file_imported_symbol_relationships
                .len(),
            total_definition_relationships: graph_data.definition_relationships.len(),
            total_definition_imported_symbol_relationships: graph_data
                .definition_imported_symbol_relationships
                .len(),
            analysis_duration,
            files_by_language,
            definitions_by_type,
            imported_symbols_by_type,
            relationships_by_type,
        }
    }

    /// Format statistics as a readable string
    pub fn format_stats(&self) -> String {
        let mut result = String::new();
        result.push_str(&format!(
            "📊 Analysis Statistics (completed in {:?}):\n",
            self.analysis_duration
        ));
        result.push_str(&format!(
            "  • Directories created: {}\n",
            self.total_directories_created
        ));
        result.push_str(&format!(
            "  • Files analyzed: {}\n",
            self.total_files_analyzed
        ));
        result.push_str(&format!(
            "  • Definitions created: {}\n",
            self.total_definitions_created
        ));
        result.push_str(&format!(
            "  • Imported symbols created: {}\n",
            self.total_imported_symbols_created
        ));
        result.push_str(&format!(
            "  • Directory relationships: {}\n",
            self.total_directory_relationships
        ));
        result.push_str(&format!(
            "  • File-definition relationships: {}\n",
            self.total_file_definition_relationships
        ));
        result.push_str(&format!(
            "  • File-imported-symbol relationships: {}\n",
            self.total_file_imported_symbol_relationships
        ));
        result.push_str(&format!(
            "  • Definition relationships: {}\n",
            self.total_definition_relationships
        ));

        if !self.files_by_language.is_empty() {
            result.push_str("  • Files by language:\n");
            for (language, count) in &self.files_by_language {
                result.push_str(&format!("    - {language}: {count}\n"));
            }
        }

        if !self.definitions_by_type.is_empty() {
            result.push_str("  • Definitions by type:\n");
            for (def_type, count) in &self.definitions_by_type {
                result.push_str(&format!("    - {def_type}: {count}\n"));
            }
        }

        if !self.imported_symbols_by_type.is_empty() {
            result.push_str("  • Imported symbols by type:\n");
            for (imported_symbol_type, count) in &self.imported_symbols_by_type {
                result.push_str(&format!("    - {imported_symbol_type}: {count}\n"));
            }
        }

        if !self.relationships_by_type.is_empty() {
            result.push_str("  • Relationships by type:\n");
            for (rel_type, count) in &self.relationships_by_type {
                let rel_type_str = rel_type.as_str();
                result.push_str(&format!("    - {rel_type_str}: {count}\n"));
            }
        }

        result
    }
}
