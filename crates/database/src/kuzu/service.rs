use crate::graph::RelationshipType;
use crate::kuzu::types::{FromKuzuNode, KuzuNodeType, QueryNoop, QuoteEscape};
use crate::kuzu::types::{NodeCounts, RelationshipCounts};
use crate::kuzu::{connection::KuzuConnection, types::DatabaseError};
use crate::querying::query_builder::QueryBuilder;
use anyhow::Error;
use kuzu::Database;
use std::collections::HashMap;

pub struct NodeDatabaseService<'a> {
    database: &'a Database,
    query_builder: QueryBuilder,
    pub transaction_conn: Option<KuzuConnection<'a>>,
}

impl<'a> NodeDatabaseService<'a> {
    pub fn new(database: &'a Database) -> Self {
        let query_builder = QueryBuilder::new();
        Self {
            database,
            query_builder,
            transaction_conn: None,
        }
    }

    pub fn new_with_transaction(database: &'a Database) -> Self {
        let query_builder = QueryBuilder::new();
        let transaction_conn = KuzuConnection::new(database).unwrap();
        Self {
            database,
            query_builder,
            transaction_conn: Some(transaction_conn),
        }
    }

    pub fn transaction(
        &mut self,
        f: impl FnOnce(&mut NodeDatabaseService) -> Result<(), DatabaseError>,
    ) -> Result<(), DatabaseError> {
        if self.transaction_conn.is_none() {
            return Err(DatabaseError::Kuzu(kuzu::Error::FailedQuery(
                "No transaction connection available".to_string(),
            )));
        }
        f(self)?;
        Ok(())
    }

    // HELPERS
    fn get_connection(&self) -> KuzuConnection<'_> {
        match KuzuConnection::new(self.database) {
            Ok(connection) => connection,
            Err(connection_error) => {
                panic!("Failed to create database connection: {connection_error}");
            }
        }
    }

    fn iter_query_result<R: FromKuzuNode>(&self, query_result: kuzu::QueryResult) -> Vec<R> {
        let mut nodes = Vec::new();
        for row in query_result {
            nodes.push(R::from_kuzu_node(row.first().unwrap()));
        }
        nodes
    }

    fn get_scalar_query_result(&self, mut result: kuzu::QueryResult) -> Option<u64> {
        result.next()?.first().and_then(|value| match value {
            kuzu::Value::Int64(v) => Some(*v as u64),
            kuzu::Value::UInt32(v) => Some(*v as u64),
            _ => None,
        })
    }

    fn get_scalar_query_results(
        &self,
        mut result: kuzu::QueryResult,
        columns: Vec<&str>,
    ) -> Option<HashMap<String, u64>> {
        result.next().map(|row| {
            columns
                .into_iter()
                .enumerate()
                .filter_map(|(i, name)| {
                    row.get(i).and_then(|value| match value {
                        kuzu::Value::Int64(v) => Some((name.to_string(), *v as u64)),
                        kuzu::Value::UInt32(v) => Some((name.to_string(), *v as u64)),
                        _ => None,
                    })
                })
                .collect()
        })
    }

    // COMMANDS

    /// Delete nodes from a table by a column value
    pub fn delete_by<T: std::fmt::Display + QuoteEscape>(
        &self,
        node_type: KuzuNodeType,
        column: &str,
        values: &[T],
    ) -> Result<(), DatabaseError> {
        match self.query_builder.delete_by(node_type, column, values) {
            (QueryNoop::No, query) => {
                self.query_builder.log_query(&query);
                match self.transaction_conn {
                    Some(ref conn) => conn.execute_ddl(&query)?,
                    None => self.get_connection().execute_ddl(&query)?,
                }
                Ok(())
            }
            (QueryNoop::Yes, _) => Ok(()),
        }
    }

    pub fn get_by<T: std::fmt::Display + QuoteEscape, R: FromKuzuNode>(
        &self,
        node_type: KuzuNodeType,
        column: &str,
        values: &[T],
    ) -> Result<Vec<R>, DatabaseError> {
        match self.query_builder.get_by::<T, R>(node_type, column, values) {
            (QueryNoop::No, query) => {
                self.query_builder.log_query(&query);
                if let Some(ref conn) = self.transaction_conn {
                    let result = conn.query(&query)?;
                    Ok(self.iter_query_result(result))
                } else {
                    let connection = self.get_connection();
                    let result = connection.query(&query)?;
                    Ok(self.iter_query_result(result))
                }
            }
            (QueryNoop::Yes, _) => Ok(Vec::new()),
        }
    }

    pub fn agg_node_by<R: FromKuzuNode>(
        &self,
        agg_func: &str,
        field: &str,
    ) -> Result<u64, Option<DatabaseError>> {
        let (_, query) = self.query_builder.agg_node_by::<R>(agg_func, field);
        self.query_builder.log_query(&query);
        if let Some(ref conn) = self.transaction_conn {
            match conn.query(&query) {
                Ok(result) => self.get_scalar_query_result(result).ok_or(None),
                Err(e) => Err(Some(e)),
            }
        } else {
            let connection = self.get_connection();
            match connection.query(&query) {
                Ok(result) => self.get_scalar_query_result(result).ok_or(None),
                Err(e) => Err(Some(e)),
            }
        }
    }

    pub fn count_nodes<R: FromKuzuNode>(&self) -> u64 {
        let (_, query) = self.query_builder.count_nodes::<R>();
        let connection = self.get_connection();
        self.query_builder.log_query(&query);
        match connection.query(&query) {
            Ok(result) => self.get_scalar_query_result(result).unwrap_or(0),
            Err(_) => 0,
        }
    }

    pub fn count_node_by<T: std::fmt::Display + QuoteEscape>(
        &self,
        node_type: KuzuNodeType,
        field: &str,
        values: &[T],
    ) -> Result<i64, Option<DatabaseError>> {
        match self
            .query_builder
            .count_nodes_by::<T>(node_type, field, values)
        {
            (QueryNoop::Yes, _) => Ok(0),
            (QueryNoop::No, query) => {
                let connection = self.get_connection();
                let result = connection.query(&query)?;
                self.get_scalar_query_result(result)
                    .map(|v| v as i64)
                    .ok_or(None)
            }
        }
    }

    pub fn get_all<R: FromKuzuNode>(
        &self,
        kuzu_node_type: KuzuNodeType,
    ) -> Result<Vec<R>, DatabaseError> {
        match self.query_builder.get_all::<R>(kuzu_node_type) {
            (QueryNoop::Yes, _) => Ok(Vec::new()),
            (QueryNoop::No, query) => {
                let connection = self.get_connection();
                self.query_builder.log_query(&query);
                let result = connection.query(&query)?;
                Ok(self.iter_query_result(result))
            }
        }
    }

    /// Get node counts (for database verification)
    pub fn get_node_counts(&self) -> Result<NodeCounts, Error> {
        let connection = self.get_connection();
        let (_, query) = self.query_builder.get_node_counts();
        self.query_builder.log_query(&query);
        match connection.query(&query) {
            Ok(result) => {
                match self.get_scalar_query_results(
                    result,
                    vec!["dir_count", "file_count", "def_count", "imp_count"],
                ) {
                    Some(counts) => Ok(NodeCounts {
                        directory_count: counts["dir_count"] as u32,
                        file_count: counts["file_count"] as u32,
                        definition_count: counts["def_count"] as u32,
                        imported_symbol_count: counts["imp_count"] as u32,
                    }),
                    None => Err(Error::msg("No node counts found")),
                }
            }
            Err(_) => Err(Error::msg("No node counts found")),
        }
    }

    /// Get relationship counts (for database verification)
    pub fn get_relationship_counts(&self) -> Result<RelationshipCounts, Error> {
        let connection = self.get_connection();
        let (_, query) = self.query_builder.get_relationship_counts();
        self.query_builder.log_query(&query);
        match connection.query(&query) {
            Ok(result) => {
                match self.get_scalar_query_results(
                    result,
                    vec![
                        "dir_rel_count",
                        "file_rel_count",
                        "def_rel_count",
                        "imp_rel_count",
                    ],
                ) {
                    Some(counts) => Ok(RelationshipCounts {
                        directory_relationships: counts["dir_rel_count"] as u32,
                        file_relationships: counts["file_rel_count"] as u32,
                        definition_relationships: counts["def_rel_count"] as u32,
                        imported_symbol_relationships: counts["imp_rel_count"] as u32,
                    }),
                    None => Err(Error::msg("No relationship counts found")),
                }
            }
            Err(_) => Err(Error::msg("No relationship counts found")),
        }
    }

    /// Count relationships of a specific type
    pub fn count_relationships_of_type(&self, relationship_type: RelationshipType) -> i64 {
        let connection = self.get_connection();
        let (_, query) = self
            .query_builder
            .count_relationships_of_type(relationship_type);
        self.query_builder.log_query(&query);
        match connection.query(&query) {
            Ok(result) => self
                .get_scalar_query_result(result)
                .map(|v| v as i64)
                .unwrap_or(0),
            Err(_) => 0,
        }
    }

    /// Count relationships of a specific node type
    pub fn count_relationships_of_node_type(&self, node_type: KuzuNodeType) -> i64 {
        let connection = self.get_connection();
        let (_, query) = self
            .query_builder
            .count_relationships_of_node_type(node_type);
        self.query_builder.log_query(&query);
        match connection.query(&query) {
            Ok(result) => self
                .get_scalar_query_result(result)
                .map(|v| v as i64)
                .unwrap_or(0),
            Err(_) => 0,
        }
    }

    /// Find call relationships from a source method to a target method
    pub fn find_call_relationships(
        &self,
        source_fqn: &str,
        target_fqn: &str,
    ) -> Result<Vec<(String, String, String)>, DatabaseError> {
        let query =
            format!(
            "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode) 
             WHERE source.fqn = '{}' AND target.fqn = '{}' AND r.type = '{}' 
             RETURN source.fqn, target.fqn, r.type",
            source_fqn, target_fqn, RelationshipType::Calls.as_str()
        );

        let conn = self.get_connection();
        let result = conn.query(&query)?;

        let mut relationships = Vec::new();
        for row in result {
            if let (
                Some(kuzu::Value::String(source)),
                Some(kuzu::Value::String(target)),
                Some(kuzu::Value::String(rel_type)),
            ) = (row.first(), row.get(1), row.get(2))
            {
                relationships.push((source.to_string(), target.to_string(), rel_type.to_string()));
            }
        }

        Ok(relationships)
    }

    /// Find all method calls made by a specific method
    pub fn find_calls_from_method(&self, source_fqn: &str) -> Result<Vec<String>, DatabaseError> {
        let query = format!(
            "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode) 
             WHERE source.fqn = '{}' AND r.type = '{}' 
             RETURN target.fqn",
            source_fqn,
            RelationshipType::Calls.as_str()
        );

        let conn = self.get_connection();
        let result = conn.query(&query)?;

        let mut target_fqns = Vec::new();
        for row in result {
            if let Some(kuzu::Value::String(target_fqn)) = row.first() {
                target_fqns.push(target_fqn.to_string());
            }
        }

        Ok(target_fqns)
    }

    /// Find all methods that call a specific target method
    pub fn find_calls_to_method(&self, target_fqn: &str) -> Result<Vec<String>, DatabaseError> {
        let query = format!(
            "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode) 
             WHERE target.fqn = '{}' AND r.type = '{}' 
             RETURN source.fqn",
            target_fqn,
            RelationshipType::Calls.as_str()
        );

        let conn = self.get_connection();
        let result = conn.query(&query)?;

        let mut source_fqns = Vec::new();
        for row in result {
            if let Some(kuzu::Value::String(source_fqn)) = row.first() {
                source_fqns.push(source_fqn.to_string());
            }
        }

        Ok(source_fqns)
    }

    pub fn find_calls_to_imported_symbol(
        &self,
        import_path: &str,
        import_name: &str,
    ) -> Result<Vec<String>, DatabaseError> {
        let query = format!(
            "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:ImportedSymbolNode) 
             WHERE target.import_path = '{}' AND target.name = '{}' AND r.type = '{}' 
             RETURN source.fqn",
            import_path,
            import_name,
            RelationshipType::Calls.as_str()
        );

        let conn = self.get_connection();
        let result = conn.query(&query)?;

        let mut source_fqns = Vec::new();
        for row in result {
            if let Some(kuzu::Value::String(source_fqn)) = row.first() {
                source_fqns.push(source_fqn.to_string());
            }
        }

        Ok(source_fqns)
    }

    /// Find all methods that call a specific target method
    pub fn find_n_first_calls_to_method(
        &self,
        target_fqn: &str,
        limit: u32,
    ) -> Result<Vec<String>, DatabaseError> {
        let query = format!(
            "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode) 
             WHERE target.fqn = '{}' AND r.type = '{}' 
             RETURN source.fqn
             LIMIT {}",
            target_fqn,
            RelationshipType::Calls.as_str(),
            limit
        );

        let conn = self.get_connection();
        let result = conn.query(&query)?;

        let mut source_fqns = Vec::new();
        for row in result {
            if let Some(kuzu::Value::String(source_fqn)) = row.first() {
                source_fqns.push(source_fqn.to_string());
            }
        }

        Ok(source_fqns)
    }

    /// Count total call relationships
    pub fn count_call_relationships(&self) -> i64 {
        let query = format!(
            "MATCH ()-[r:DEFINITION_RELATIONSHIPS]->() WHERE r.type = '{}' RETURN count(r)",
            RelationshipType::Calls.as_str()
        );

        let conn = self.get_connection();
        match conn.query(&query) {
            Ok(result) => self.get_scalar_query_result(result).unwrap_or(0) as i64,
            Err(_) => 0,
        }
    }

    /// Get all call relationships for debugging
    pub fn get_all_call_relationships(
        &self,
    ) -> Result<Vec<(String, String, String)>, DatabaseError> {
        let query = format!(
            "MATCH (source:DefinitionNode)-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode) 
             WHERE r.type = '{}' 
             RETURN source.fqn, target.fqn, r.type 
             LIMIT 50",
            RelationshipType::Calls.as_str()
        );

        let conn = self.get_connection();
        let result = conn.query(&query)?;

        let mut call_relationships = Vec::new();
        for row in result {
            if let (
                Some(kuzu::Value::String(source_fqn)),
                Some(kuzu::Value::String(target_fqn)),
                Some(kuzu::Value::String(rel_type)),
            ) = (row.first(), row.get(1), row.get(2))
            {
                call_relationships.push((
                    source_fqn.to_string(),
                    target_fqn.to_string(),
                    rel_type.to_string(),
                ));
            }
        }

        Ok(call_relationships)
    }
}
