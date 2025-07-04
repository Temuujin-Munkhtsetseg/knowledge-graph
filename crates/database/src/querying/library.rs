use serde_json::Value;
use std::collections::HashMap;

pub struct QueryLibrary;

#[derive(Debug, Clone)]
pub struct Query {
    pub name: &'static str,
    pub slug: &'static str,
    pub description: &'static str,
    pub query: &'static str,
    pub parameters: HashMap<&'static str, QueryParameter>,
}

#[derive(Debug, Clone)]
pub struct QueryParameter {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub kind: QueryParameterKind,
    pub default: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum QueryParameterKind {
    String,
    Int,
    Float,
    Boolean,
}

impl QueryLibrary {
    pub fn get_neighbours_query() -> Query {
        Query {
            name: "Get Neighbours",
            slug: "get_neighbours",
            description: "Get all neighbours of a definition.",
            query: r#"
                MATCH (n)-[r]-(neighbor)
                WHERE n.fqn = $fqn
                RETURN 
                    n.fqn,
                    n.name,
                    n.definition_type,
                    n.primary_file_path,
                    n.primary_line_number,
                    r.type as relationship_type,
                    neighbor.fqn as neighbor_fqn,
                    neighbor.name as neighbor_name,
                    neighbor.definition_type as neighbor_type,
                    neighbor.primary_file_path as neighbor_file,
                    neighbor.primary_line_number as neighbor_line
                LIMIT $limit
            "#,
            parameters: HashMap::from([
                (
                    "fqn",
                    QueryParameter {
                        name: "fqn",
                        description: "The FQN of the definition to get neighbours for.",
                        required: true,
                        kind: QueryParameterKind::String,
                        default: None,
                    },
                ),
                (
                    "limit",
                    QueryParameter {
                        name: "limit",
                        description: "The maximum number of neighbours to return, defaults to 10.",
                        required: false,
                        kind: QueryParameterKind::Int,
                        default: Some(Value::Number(10.into())),
                    },
                ),
            ]),
        }
    }

    pub fn all_queries() -> Vec<Query> {
        vec![Self::get_neighbours_query()]
    }
}
