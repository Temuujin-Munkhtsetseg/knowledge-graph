use crate::graph::{RelationshipType, RelationshipTypeMapping};
use crate::kuzu::types::{
    FromKuzuNode, KuzuNodeType, QueryGeneratorResult, QueryNoop, QuoteEscape,
};
use tracing::info;

#[derive(Default)]
pub struct QueryBuilder {
    log_queries: bool,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self { log_queries: false }
    }

    // HELPERS
    fn build_values_str<T: std::fmt::Display + QuoteEscape>(&self, values: &[T]) -> String {
        values
            .iter()
            .map(|val| {
                if val.needs_quotes() {
                    format!("'{val}'")
                } else {
                    format!("{val}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn log_query(&self, query: &str) {
        if self.log_queries {
            info!("Query: {query}");
        }
    }

    pub fn delete_by<T: std::fmt::Display + QuoteEscape>(
        &self,
        node_type: KuzuNodeType,
        column: &str,
        values: &[T],
    ) -> QueryGeneratorResult {
        if values.is_empty() {
            return (QueryNoop::Yes, String::new());
        }
        let values_str = self.build_values_str(values);
        (
            QueryNoop::No,
            format!(
                "MATCH (n:{}) WHERE n.{column} IN [{values_str}] DETACH DELETE n",
                node_type.as_str(),
            ),
        )
    }

    pub fn get_by<T: std::fmt::Display + QuoteEscape, R: FromKuzuNode>(
        &self,
        node_type: KuzuNodeType,
        column: &str,
        values: &[T],
    ) -> QueryGeneratorResult {
        if values.is_empty() {
            return (QueryNoop::Yes, String::new());
        }
        let values_str = self.build_values_str(values);
        (
            QueryNoop::No,
            format!(
                "MATCH (n:{}) WHERE n.{column} IN [{values_str}] RETURN n",
                node_type.as_str(),
            ),
        )
    }

    pub fn agg_node_by<R: FromKuzuNode>(
        &self,
        agg_func: &str,
        field: &str,
    ) -> QueryGeneratorResult {
        (
            QueryNoop::No,
            format!("MATCH (n:{}) RETURN {}(n.{})", R::name(), agg_func, field),
        )
    }

    pub fn count_nodes<R: FromKuzuNode>(&self) -> QueryGeneratorResult {
        (
            QueryNoop::No,
            format!("MATCH (n:{}) RETURN COUNT(n)", R::name()),
        )
    }

    pub fn count_nodes_by<T: std::fmt::Display + QuoteEscape>(
        &self,
        node_type: KuzuNodeType,
        field: &str,
        values: &[T],
    ) -> QueryGeneratorResult {
        if values.is_empty() {
            return (QueryNoop::Yes, String::new());
        }
        let values_str = self.build_values_str(values);
        (
            QueryNoop::No,
            format!(
                "MATCH (n:{}) WHERE n.{field} IN [{values_str}] RETURN COUNT(n)",
                node_type.as_str()
            ),
        )
    }

    pub fn get_all<R: FromKuzuNode>(&self, kuzu_node_type: KuzuNodeType) -> QueryGeneratorResult {
        (
            QueryNoop::No,
            format!("MATCH (n:{}) RETURN n", kuzu_node_type.as_str()),
        )
    }

    pub fn get_node_counts(&self) -> QueryGeneratorResult {
        (
            QueryNoop::No,
            "
            OPTIONAL MATCH (d:DirectoryNode) 
            WITH count(d) as dir_count
            OPTIONAL MATCH (f:FileNode) 
            WITH dir_count, count(f) as file_count
            OPTIONAL MATCH (def:DefinitionNode)
            WITH dir_count, file_count, count(def) as def_count
            OPTIONAL MATCH (imp:ImportedSymbolNode)
            RETURN dir_count, file_count, def_count, count(imp) as imp_count
        "
            .to_string(),
        )
    }

    pub fn get_relationship_counts(&self) -> QueryGeneratorResult {
        (
            QueryNoop::No,
            "
            OPTIONAL MATCH ()-[d:DIRECTORY_RELATIONSHIPS]->()
            WITH count(d) as dir_rel_count
            OPTIONAL MATCH ()-[f:FILE_RELATIONSHIPS]->()
            WITH dir_rel_count, count(f) as file_rel_count
            OPTIONAL MATCH ()-[def:DEFINITION_RELATIONSHIPS]->()
            RETURN dir_rel_count, file_rel_count, count(def) as def_rel_count
        "
            .to_string(),
        )
    }

    pub fn count_relationships_of_type(
        &self,
        relationship_type: RelationshipType,
    ) -> QueryGeneratorResult {
        // Get the relationship label based on the type
        let (rel_label, _type_id) = match relationship_type {
            RelationshipType::DirContainsDir | RelationshipType::DirContainsFile => {
                ("DIRECTORY_RELATIONSHIPS", relationship_type.as_str())
            }
            RelationshipType::FileDefines | RelationshipType::FileImports => {
                ("FILE_RELATIONSHIPS", relationship_type.as_str())
            }
            _ => {
                // All other types are definition relationships
                ("DEFINITION_RELATIONSHIPS", relationship_type.as_str())
            }
        };

        (
            QueryNoop::No,
            format!(
                "MATCH (from)-[r:{}]->(to) WHERE r.type = {} RETURN COUNT(DISTINCT [from, to])",
                rel_label,
                RelationshipTypeMapping::new().get_type_id(relationship_type)
            ),
        )
    }

    pub fn count_relationships_of_node_type(
        &self,
        node_type: KuzuNodeType,
    ) -> QueryGeneratorResult {
        // Get the relationship label based on the type
        let (rel_label, _type_id) = match node_type {
            KuzuNodeType::DirectoryNode => ("DIRECTORY_RELATIONSHIPS", node_type.as_str()),
            KuzuNodeType::FileNode => ("FILE_RELATIONSHIPS", node_type.as_str()),
            KuzuNodeType::DefinitionNode => ("DEFINITION_RELATIONSHIPS", node_type.as_str()),
            KuzuNodeType::ImportedSymbolNode => todo!(),
        };
        (
            QueryNoop::No,
            format!("MATCH (from)-[r:{rel_label}]->(to) RETURN COUNT(DISTINCT [from, to])"),
        )
    }

    pub fn copy_nodes_from_parquet(
        &self,
        table_name: &str,
        file_path: &str,
    ) -> QueryGeneratorResult {
        (
            QueryNoop::No,
            format!("COPY {table_name} FROM '{file_path}' (FORMAT 'parquet')"),
        )
    }

    pub fn copy_relationships_from_parquet(
        &self,
        table_name: &str,
        file_path: &str,
        from_table: Option<&str>,
        to_table: Option<&str>,
    ) -> QueryGeneratorResult {
        let mut query = format!("COPY {table_name} FROM '{file_path}'");

        // For Parquet files, only from/to options are needed for relationship tables
        // HEADER is not needed as schema is embedded in Parquet metadata
        let mut options = Vec::new();
        if let Some(from) = from_table {
            options.push(format!("from='{from}'"));
        }

        if let Some(to) = to_table {
            options.push(format!("to='{to}'"));
        }

        if !options.is_empty() {
            let options_str: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
            query.push_str(&format!(" ({})", options_str.join(", ")));
        }

        (QueryNoop::No, query)
    }
}
