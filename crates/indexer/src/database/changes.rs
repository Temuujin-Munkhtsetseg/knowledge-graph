use database::kuzu::schema::{SchemaManager, SchemaManagerImportMode};
use kuzu::Database;

use crate::analysis::{DefinitionNode, GraphData};
use crate::database::types::FromKuzuNode;
use crate::database::types::*;
use crate::database::utils::NodeIdGenerator;
use crate::node_database_service::NodeDatabaseService;
use crate::parsing::changes::{FileChanges, FileChangesPathType};
use crate::writer::{WriterResult, WriterService};

// HELPERS
fn flatten_definitions(graph_data: &GraphData) -> Vec<DefinitionNode> {
    let mut flattened_definitions: Vec<DefinitionNode> = Vec::new();
    for def in &graph_data.definition_nodes {
        for file_location in &def.file_locations {
            let flattened_definition = DefinitionNode {
                fqn: def.fqn.clone(),
                definition_type: def.definition_type,
                name: def.name.clone(),
                file_locations: vec![file_location.clone()],
            };
            flattened_definitions.push(flattened_definition);
        }
    }
    flattened_definitions
}

#[derive(Debug, Clone)]
pub struct KuzuNodeChanges<T> {
    pub modified_nodes: Vec<T>,
    pub added_nodes: Vec<T>,
}

pub struct KuzuChanges<'a> {
    pub database: &'a Database,
    pub node_database_service: NodeDatabaseService<'a>,
    pub file_changes: FileChanges,
    pub graph_data: GraphData,
    pub repo_path: String,
    pub output_path: String,
}

impl<'a> KuzuChanges<'a> {
    pub fn new(
        database: &'a Database,
        file_changes: FileChanges,
        graph_data: GraphData,
        repo_path: &str,
        output_path: &str,
    ) -> Self {
        Self {
            database,
            node_database_service: NodeDatabaseService::new(database),
            file_changes,
            graph_data,
            repo_path: repo_path.to_string(),
            output_path: output_path.to_string(),
        }
    }

    pub fn sync_changes(&mut self) -> WriterResult {
        // First, delete the old nodes and their relationships
        self.apply_destructive_changes();

        // Get the new node ID heads
        let (max_definition_id, max_file_id, max_dir_id) = self.new_node_id_heads();
        let mut node_id_generator = NodeIdGenerator::new();
        node_id_generator.next_definition_id = max_definition_id + 1;
        node_id_generator.next_file_id = max_file_id + 1;
        node_id_generator.next_directory_id = max_dir_id + 1;

        // Clear the ID mappings to ensure new IDs are assigned
        node_id_generator.clear();

        // Write new nodes to Parquet files with new IDs
        let writer_service = WriterService::new(&self.output_path)
            .map_err(|e| format!("Failed to create writer service: {e}"))
            .unwrap();

        writer_service.flush_output_directory();

        let result = writer_service
            .write_graph_data(&self.graph_data, &mut node_id_generator)
            .map_err(|e| format!("Writing failed: {e}"))
            .expect("Failed to write graph data");

        // Delete the nodes for changed files and directories from the database
        let changed_files = self
            .file_changes
            .get_rel_paths(FileChangesPathType::ChangedFiles, &self.repo_path);

        let _ = self.node_database_service.delete_by(
            KuzuNodeType::DefinitionNode,
            "primary_file_path",
            &changed_files,
        );

        let _ =
            self.node_database_service
                .delete_by(KuzuNodeType::FileNode, "path", &changed_files);

        // Delete the nodes for deleted files from the database
        let changed_dirs = self
            .file_changes
            .get_rel_paths(FileChangesPathType::ChangedDirs, &self.repo_path);

        let _ = self.node_database_service.delete_by(
            KuzuNodeType::DirectoryNode,
            "path",
            &changed_dirs,
        );

        // Import the new nodes from Parquet files
        let schema_manager = SchemaManager::new(self.database);

        schema_manager
            .import_graph_data(&self.output_path, SchemaManagerImportMode::Reindexing)
            .expect("Failed to import graph data");

        result
    }

    fn new_node_id_heads(&mut self) -> (u32, u32, u32) {
        // Compute the max id of each node type
        let max_definition_id = self
            .node_database_service
            .agg_node_by(KuzuNodeType::DefinitionNode, "max", "id")
            .unwrap();

        let max_file_id = self
            .node_database_service
            .agg_node_by(KuzuNodeType::FileNode, "max", "id")
            .unwrap();

        let max_dir_id = self
            .node_database_service
            .agg_node_by(KuzuNodeType::DirectoryNode, "max", "id")
            .unwrap();

        (max_definition_id, max_file_id, max_dir_id)
    }

    fn find_nodes<R: FromKuzuNode>(
        &mut self,
        path_type: FileChangesPathType,
        node_type: KuzuNodeType,
    ) -> Vec<R> {
        let changed_files = self.file_changes.get_rel_paths(path_type, &self.repo_path);
        match node_type {
            KuzuNodeType::DefinitionNode => self
                .node_database_service
                .get_by::<String, R>(node_type, "primary_file_path", &changed_files)
                .unwrap(),

            KuzuNodeType::FileNode => self
                .node_database_service
                .get_by::<String, R>(node_type, "path", &changed_files)
                .unwrap(),

            KuzuNodeType::DirectoryNode => self
                .node_database_service
                .get_by::<String, R>(node_type, "path", &changed_files)
                .unwrap(),
        }
    }

    fn apply_destructive_changes(&mut self) {
        // Find removed definitions (exist in kuzu but not in new)
        let changed_def_nodes = self.find_nodes::<DefinitionNodeFromKuzu>(
            FileChangesPathType::ChangedFiles,
            KuzuNodeType::DefinitionNode,
        );
        let flattened_definitions = flatten_definitions(&self.graph_data);

        let deleted_definitions = changed_def_nodes
            .iter()
            .filter(|kuzu_def| {
                !flattened_definitions.iter().any(|def| {
                    def.fqn == kuzu_def.fqn
                        && def.file_locations[0].file_path == kuzu_def.primary_file_path
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        // Remove deleted definitions (and their relationships)
        let deleted_def_ids = deleted_definitions
            .iter()
            .map(|def| def.id)
            .collect::<Vec<_>>();
        let _ = self.node_database_service.delete_by(
            KuzuNodeType::DefinitionNode,
            "id",
            &deleted_def_ids,
        );

        // Find removed files (exist in kuzu but not in new)
        let deleted_files = self.find_nodes::<FileNodeFromKuzu>(
            FileChangesPathType::DeletedFiles,
            KuzuNodeType::FileNode,
        );

        // Remove deleted files (and their relationships)
        let deleted_file_ids = deleted_files.iter().map(|file| file.id).collect::<Vec<_>>();
        let _ =
            self.node_database_service
                .delete_by(KuzuNodeType::FileNode, "id", &deleted_file_ids);

        // Find removed directories (exist in kuzu but not in new)
        let deleted_dirs = self.find_nodes::<DirectoryNodeFromKuzu>(
            FileChangesPathType::DeletedFiles,
            KuzuNodeType::DirectoryNode,
        );

        // Remove deleted directories (and their relationships)
        let deleted_dir_ids = deleted_dirs.iter().map(|dir| dir.id).collect::<Vec<_>>();
        let _ = self.node_database_service.delete_by(
            KuzuNodeType::DirectoryNode,
            "id",
            &deleted_dir_ids,
        );
    }
}
