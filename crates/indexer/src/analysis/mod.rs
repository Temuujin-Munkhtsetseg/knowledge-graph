pub mod files;
pub mod languages;
pub mod types;

use crate::analysis::types::{
    DefinitionNode, DefinitionRelationship, DirectoryNode, DirectoryRelationship,
    FileDefinitionRelationship, FileNode, FqnType, GraphData,
};
use crate::parsing::processor::FileProcessingResult;
use parser_core::parser::SupportedLanguage;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

// Re-export the sub-module functionality
pub use files::FileSystemAnalyzer;
pub use languages::python::PythonAnalyzer;
pub use languages::ruby::RubyAnalyzer;

/// Analysis service that orchestrates the transformation of parsing results into graph data
pub struct AnalysisService {
    repository_name: String,
    repository_path: String,
    filesystem_analyzer: FileSystemAnalyzer,
    ruby_analyzer: RubyAnalyzer,
    python_analyzer: PythonAnalyzer,
}

impl AnalysisService {
    /// Create a new analysis service
    pub fn new(repository_name: String, repository_path: String) -> Self {
        let filesystem_analyzer =
            FileSystemAnalyzer::new(repository_name.clone(), repository_path.clone());
        let ruby_analyzer = RubyAnalyzer::new();
        let python_analyzer = PythonAnalyzer::new();

        Self {
            repository_name,
            repository_path,
            filesystem_analyzer,
            ruby_analyzer,
            python_analyzer,
        }
    }

    /// Analyze file processing results and transform them into graph data
    pub fn analyze_results(
        &self,
        file_results: &Vec<FileProcessingResult>,
    ) -> Result<GraphData, String> {
        let start_time = Instant::now();
        log::info!(
            "Starting analysis of {} file results for repository '{}' at '{}'",
            file_results.len(),
            self.repository_name,
            self.repository_path
        );

        let mut definition_nodes: Vec<DefinitionNode> = Vec::new();
        let mut directory_nodes: Vec<DirectoryNode> = Vec::new();
        let mut file_nodes: Vec<FileNode> = Vec::new();
        let mut directory_relationships: Vec<DirectoryRelationship> = Vec::new();
        let mut file_definition_relationships: Vec<FileDefinitionRelationship> = Vec::new();
        let mut definition_relationships: Vec<DefinitionRelationship> = Vec::new();

        // TODO: Deprecate these. Can make directory_nodes and directory_relationships HashMaps.
        let mut created_directories = HashSet::new();
        let mut created_dir_relationships = HashSet::new();

        let results_by_language = self.group_results_by_language(file_results);
        for (language, results) in results_by_language {
            let mut definition_map = HashMap::new();
            for file_result in results {
                self.extract_file_system_entities(
                    file_result,
                    &mut file_nodes,
                    &mut directory_nodes,
                    &mut directory_relationships,
                    &mut created_directories,
                    &mut created_dir_relationships,
                );
                self.extract_language_entities(
                    file_result,
                    &mut definition_map,
                    &mut file_definition_relationships,
                );
            }

            self.add_definition_nodes(&definition_map, &mut definition_nodes);
            self.add_definition_relationships(
                language,
                definition_map,
                &mut definition_relationships,
            );
        }

        let analysis_time = start_time.elapsed();
        log::info!(
            "Analysis completed in {:?}: {} directories, {} files, {} definitions ({} total locations), {} total relationships",
            analysis_time,
            directory_nodes.len(),
            file_nodes.len(),
            definition_nodes.len(),
            definition_nodes.iter().map(|_d| 1).sum::<usize>(),
            directory_relationships.len()
                + file_definition_relationships.len()
                + definition_relationships.len()
        );

        Ok(GraphData {
            directory_nodes,
            file_nodes,
            definition_nodes,
            directory_relationships,
            file_definition_relationships,
            definition_relationships,
        })
    }

    fn group_results_by_language<'a>(
        &self,
        file_results: &'a Vec<FileProcessingResult>,
    ) -> HashMap<SupportedLanguage, Vec<&'a FileProcessingResult>> {
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
                relationship_type: "DIR_CONTAINS_FILE".to_string(),
            });
        }
        file_nodes.push(file_node);
    }

    fn extract_language_entities(
        &self,
        file_result: &FileProcessingResult,
        definition_map: &mut HashMap<(String, String), (DefinitionNode, FqnType)>,
        file_definition_relationships: &mut Vec<FileDefinitionRelationship>,
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
            }
            _ => {}
        }
    }

    fn add_definition_nodes(
        &self,
        definition_map: &HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_nodes: &mut Vec<DefinitionNode>,
    ) {
        let unrolled_definitions: Vec<DefinitionNode> = definition_map
            .values()
            .map(|(def_node, _)| def_node.clone())
            .collect();
        definition_nodes.extend(unrolled_definitions);
    }

    fn add_definition_relationships(
        &self,
        language: SupportedLanguage,
        definition_map: HashMap<(String, String), (DefinitionNode, FqnType)>,
        definition_relationships: &mut Vec<DefinitionRelationship>,
    ) {
        match language {
            SupportedLanguage::Ruby => {
                self.ruby_analyzer
                    .add_definition_relationships(&definition_map, definition_relationships);
            }
            SupportedLanguage::Python => {
                self.python_analyzer
                    .add_definition_relationships(&definition_map, definition_relationships);
            }
            _ => {}
        }
    }
}

/// Analysis statistics
#[derive(Debug, Clone)]
pub struct AnalysisStats {
    pub total_directories_created: usize,
    pub total_files_analyzed: usize,
    pub total_definitions_created: usize,
    pub total_directory_relationships: usize,
    pub total_file_definition_relationships: usize,
    pub total_definition_relationships: usize,
    pub analysis_duration: Duration,
    pub files_by_language: HashMap<String, usize>,
    pub definitions_by_type: HashMap<String, usize>,
    pub relationships_by_type: HashMap<String, usize>,
}

impl AnalysisStats {
    /// Create analysis statistics from graph data
    pub fn from_graph_data(graph_data: &GraphData, analysis_duration: Duration) -> Self {
        let mut files_by_language = HashMap::new();
        let mut definitions_by_type = HashMap::new();
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

        // Count relationships by type
        for rel in &graph_data.directory_relationships {
            *relationships_by_type
                .entry(rel.relationship_type.clone())
                .or_insert(0) += 1;
        }
        for rel in &graph_data.file_definition_relationships {
            *relationships_by_type
                .entry(rel.relationship_type.clone())
                .or_insert(0) += 1;
        }
        for rel in &graph_data.definition_relationships {
            *relationships_by_type
                .entry(rel.relationship_type.clone())
                .or_insert(0) += 1;
        }

        Self {
            total_directories_created: graph_data.directory_nodes.len(),
            total_files_analyzed: graph_data.file_nodes.len(),
            total_definitions_created: graph_data.definition_nodes.len(),
            total_directory_relationships: graph_data.directory_relationships.len(),
            total_file_definition_relationships: graph_data.file_definition_relationships.len(),
            total_definition_relationships: graph_data.definition_relationships.len(),
            analysis_duration,
            files_by_language,
            definitions_by_type,
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
            "  • Directory relationships: {}\n",
            self.total_directory_relationships
        ));
        result.push_str(&format!(
            "  • File-definition relationships: {}\n",
            self.total_file_definition_relationships
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

        if !self.relationships_by_type.is_empty() {
            result.push_str("  • Relationships by type:\n");
            for (rel_type, count) in &self.relationships_by_type {
                result.push_str(&format!("    - {rel_type}: {count}\n"));
            }
        }

        result
    }
}
