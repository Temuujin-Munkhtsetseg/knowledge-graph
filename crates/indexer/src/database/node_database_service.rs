use crate::database::types::{
    FromKuzuNode, KuzuNodeType, NodeCounts, QuoteEscape, RelationshipCounts,
};
use database::graph::{RelationshipType, RelationshipTypeMapping};
use database::kuzu::{connection::KuzuConnection, types::DatabaseError};
use kuzu::Database;

pub struct NodeDatabaseService<'a> {
    database: &'a Database,
}

fn get_connection(database: &Database) -> KuzuConnection {
    match KuzuConnection::new(database) {
        Ok(connection) => connection,
        Err(connection_error) => {
            panic!("Failed to create database connection: {connection_error}");
        }
    }
}

impl<'a> NodeDatabaseService<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    /// Delete nodes from a table by a column value
    pub fn delete_by<T: std::fmt::Display + QuoteEscape>(
        &self,
        node_type: KuzuNodeType,
        column: &str,
        values: &[T],
    ) -> Result<(), DatabaseError> {
        let values_str = values
            .iter()
            .map(|val| {
                if val.needs_quotes() {
                    format!("'{val}'")
                } else {
                    format!("{val}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "MATCH (n:{}) WHERE n.{column} IN [{values_str}] DETACH DELETE n",
            node_type.as_str()
        );

        get_connection(self.database).execute_ddl(&query)?;

        Ok(())
    }

    pub fn agg_node_by(
        &self,
        node_type: KuzuNodeType,
        agg_func: &str,
        field: &str,
    ) -> Result<u32, DatabaseError> {
        let query = format!(
            "MATCH (n:{}) RETURN {}(n.{})",
            node_type.as_str(),
            agg_func,
            field
        );

        let connection = get_connection(self.database);
        let mut result = connection.query(&query)?;
        if let Some(row) = result.next() {
            if let Some(kuzu::Value::UInt32(count)) = row.first() {
                return Ok(*count);
            }
        }

        Ok(0)
    }

    pub fn get_by<T: std::fmt::Display + QuoteEscape, R: FromKuzuNode>(
        &self,
        node_type: KuzuNodeType,
        column: &str,
        values: &[T],
    ) -> Result<Vec<R>, DatabaseError> {
        let values_str = values
            .iter()
            .map(|val| {
                if val.needs_quotes() {
                    format!("'{val}'")
                } else {
                    format!("{val}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "MATCH (n:{}) WHERE n.{column} IN [{values_str}] RETURN n",
            node_type.as_str()
        );

        let connection = get_connection(self.database);
        let result = connection.query(&query)?;
        let mut nodes = Vec::new();

        for row in result {
            if let Some(node_value) = row.first() {
                nodes.push(R::from_kuzu_node(node_value));
            }
        }
        Ok(nodes)
    }

    #[cfg(test)]
    pub fn count_nodes(&self, node_type: KuzuNodeType) -> i64 {
        let connection = get_connection(self.database);
        let query = format!("MATCH (n:{}) RETURN COUNT(n)", node_type.as_str());
        let mut result = match connection.query(&query) {
            Ok(result) => result,
            Err(_) => return 0,
        };

        if let Some(row) = result.next() {
            if let Some(kuzu::Value::Int64(count)) = row.first() {
                *count
            } else {
                0
            }
        } else {
            0
        }
    }

    pub fn count_node_by<T: std::fmt::Display + QuoteEscape>(
        &self,
        node_type: KuzuNodeType,
        field: &str,
        values: &[T],
    ) -> Result<i64, DatabaseError> {
        let values_str = values
            .iter()
            .map(|val| {
                if val.needs_quotes() {
                    format!("'{val}'")
                } else {
                    format!("{val}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            "MATCH (n:{}) WHERE n.{} IN [{values_str}] RETURN COUNT(n)",
            node_type.as_str(),
            field,
        );

        let connection = get_connection(self.database);
        let mut result = connection.query(&query)?;

        if let Some(row) = result.next() {
            if let Some(kuzu::Value::Int64(count)) = row.first() {
                return Ok(*count);
            }
        }

        Ok(0)
    }

    pub fn get_all<R: FromKuzuNode>(
        &self,
        kuzu_node_type: KuzuNodeType,
    ) -> Result<Vec<R>, DatabaseError> {
        let query = format!("MATCH (n:{}) RETURN n", kuzu_node_type.as_str());
        let connection = get_connection(self.database);
        let result = connection.query(&query)?;
        let mut nodes = Vec::new();

        for row in result {
            if let Some(node_value) = row.first() {
                nodes.push(R::from_kuzu_node(node_value));
            }
        }
        Ok(nodes)
    }

    /// Get node counts (for database verification)
    pub fn get_node_counts(&self) -> Result<NodeCounts, DatabaseError> {
        let connection = get_connection(self.database);
        let mut directory_count = 0;
        let mut file_count = 0;
        let definition_count = 0;

        // Count directory nodes
        let query = "MATCH (n:DirectoryNode) RETURN count(n)";
        if let Ok(mut result) = connection.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    directory_count = *count as u32;
                }
            }
        }

        // Count file nodes
        let query = "MATCH (n:FileNode) RETURN count(n)";
        if let Ok(mut result) = connection.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    file_count = *count as u32;
                }
            }
        }

        Ok(NodeCounts {
            directory_count,
            file_count,
            definition_count,
        })
    }

    /// Get relationship counts (for database verification)
    pub fn get_relationship_counts(&self) -> Result<RelationshipCounts, DatabaseError> {
        let connection = get_connection(self.database);
        let mut directory_relationships = 0;
        let mut file_relationships = 0;
        let mut definition_relationships = 0;

        // Count directory relationships
        let query = "MATCH ()-[r:DIRECTORY_RELATIONSHIPS]->() RETURN count(r)";
        if let Ok(mut result) = connection.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    directory_relationships = *count as u32;
                }
            }
        }

        // Count file relationships
        let query = "MATCH ()-[r:FILE_RELATIONSHIPS]->() RETURN count(r)";
        if let Ok(mut result) = connection.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    file_relationships = *count as u32;
                }
            }
        }

        // Count definition relationships
        let query = "MATCH ()-[r:DEFINITION_RELATIONSHIPS]->() RETURN count(r)";
        if let Ok(mut result) = connection.query(query) {
            if let Some(row) = result.next() {
                if let Some(kuzu::Value::Int64(count)) = row.first() {
                    definition_relationships = *count as u32;
                }
            }
        }

        Ok(RelationshipCounts {
            directory_relationships,
            file_relationships,
            definition_relationships,
        })
    }

    /// Count relationships of a specific type
    pub fn count_relationships_of_type(&self, relationship_type: RelationshipType) -> i64 {
        let connection = get_connection(self.database);

        // Get the relationship label based on the type
        let (rel_label, _type_id) = match relationship_type {
            RelationshipType::DirContainsDir | RelationshipType::DirContainsFile => {
                ("DIRECTORY_RELATIONSHIPS", relationship_type.as_str())
            }
            RelationshipType::FileDefines => ("FILE_RELATIONSHIPS", relationship_type.as_str()),
            _ => {
                // All other types are definition relationships
                ("DEFINITION_RELATIONSHIPS", relationship_type.as_str())
            }
        };

        let query = format!(
            "MATCH (from)-[r:{}]->(to) WHERE r.type = {} RETURN COUNT(DISTINCT [from, to])",
            rel_label,
            RelationshipTypeMapping::new().get_type_id(relationship_type)
        );

        let mut result = match connection.query(&query) {
            Ok(result) => result,
            Err(_) => return 0,
        };

        if let Some(row) = result.next() {
            if let Some(kuzu::Value::Int64(count)) = row.first() {
                *count
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Count relationships of a specific node type
    pub fn count_relationships_of_node_type(&self, node_type: KuzuNodeType) -> i64 {
        let connection = get_connection(self.database);

        // Get the relationship label based on the type
        let (rel_label, _type_id) = match node_type {
            KuzuNodeType::DirectoryNode => ("DIRECTORY_RELATIONSHIPS", node_type.as_str()),
            KuzuNodeType::FileNode => ("FILE_RELATIONSHIPS", node_type.as_str()),
            KuzuNodeType::DefinitionNode => ("DEFINITION_RELATIONSHIPS", node_type.as_str()),
        };

        let query = format!("MATCH (from)-[r:{rel_label}]->(to) RETURN COUNT(DISTINCT [from, to])");

        let mut result = match connection.query(&query) {
            Ok(result) => result,
            Err(_) => return 0,
        };

        if let Some(row) = result.next() {
            if let Some(kuzu::Value::Int64(count)) = row.first() {
                *count
            } else {
                0
            }
        } else {
            0
        }
    }
}
