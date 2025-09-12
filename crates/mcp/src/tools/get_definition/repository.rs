use database::kuzu::connection::KuzuConnection;
use rmcp::model::ErrorCode;
use serde_json::{Map, Value};

// This is for intermediate representation from the database
#[derive(Debug)]
pub struct RawHit {
    pub target_type: String,
    pub id: String,
    pub name: String,
    pub fqn: String,
    pub path: String,
    pub start_line_db: i64,
    pub end_line_db: i64,
    pub rel_start_col: i64,
    pub rel_end_col: i64,
    pub rel_type_id: i64,
}

pub fn find_definitions(
    conn: &KuzuConnection,
    relative_file_path: &str,
    db_line: i64,
    start_col: i64,
    end_col: i64,
    calls_type_id: i64,
    ambiguous_calls_type_id: i64,
) -> Result<Vec<RawHit>, rmcp::ErrorData> {
    let mut base_params = Map::new();
    base_params.insert(
        "primary_file_path".to_string(),
        Value::String(relative_file_path.to_string()),
    );
    base_params.insert(
        "calls_type_ids".to_string(),
        Value::Array(vec![
            Value::from(calls_type_id),
            Value::from(ambiguous_calls_type_id),
        ]),
    );
    base_params.insert(
        "source_lines".to_string(),
        Value::Array(vec![Value::from(db_line)]),
    );
    base_params.insert("start_col".to_string(), Value::from(start_col));
    base_params.insert("end_col".to_string(), Value::from(end_col));

    let q_def_from_def = r#"
        MATCH (source:DefinitionNode {primary_file_path: $primary_file_path})-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode)
        WHERE r.type IN $calls_type_ids
          AND r.source_start_line IN $source_lines
          AND r.source_start_col <= $start_col AND r.source_end_col >= $end_col
        RETURN
          'Definition' as target_type,
          CAST(target.id AS INT64) as target_id,
          target.name as name,
          target.fqn as fqn,
          target.primary_file_path as path,
          CAST(target.start_line AS INT64) as start_line,
          CAST(target.end_line AS INT64) as end_line,
          CAST(r.source_start_col AS INT64) as rel_start_col,
          CAST(r.source_end_col AS INT64) as rel_end_col,
          CAST(r.type AS INT64) as rel_type
        LIMIT 100
    "#;

    let q_imp_from_def = r#"
        MATCH (source:DefinitionNode {primary_file_path: $primary_file_path})-[r:DEFINITION_RELATIONSHIPS]->(target:ImportedSymbolNode)
        WHERE r.type IN $calls_type_ids
          AND r.source_start_line IN $source_lines
          AND r.source_start_col <= $start_col AND r.source_end_col >= $end_col
        RETURN
          'ImportedSymbol' as target_type,
          CAST(target.id AS INT64) as target_id,
          COALESCE(target.name, '') as name,
          '' as fqn,
          target.file_path as path,
          CAST(target.start_line AS INT64) as start_line,
          CAST(target.end_line AS INT64) as end_line,
          CAST(r.source_start_col AS INT64) as rel_start_col,
          CAST(r.source_end_col AS INT64) as rel_end_col,
          CAST(r.type AS INT64) as rel_type
        LIMIT 100
    "#;

    let q_def_from_file = r#"
        MATCH (file:FileNode {path: $primary_file_path})-[r:DEFINITION_RELATIONSHIPS]->(target:DefinitionNode)
        WHERE r.type IN $calls_type_ids
          AND r.source_start_line IN $source_lines
          AND r.source_start_col <= $start_col AND r.source_end_col >= $end_col
        RETURN
          'Definition' as target_type,
          CAST(target.id AS INT64) as target_id,
          target.name as name,
          target.fqn as fqn,
          target.primary_file_path as path,
          CAST(target.start_line AS INT64) as start_line,
          CAST(target.end_line AS INT64) as end_line,
          CAST(r.source_start_col AS INT64) as rel_start_col,
          CAST(r.source_end_col AS INT64) as rel_end_col,
          CAST(r.type AS INT64) as rel_type
        LIMIT 100
    "#;

    let q_imp_from_file = r#"
        MATCH (file:FileNode {path: $primary_file_path})-[r:DEFINITION_RELATIONSHIPS]->(target:ImportedSymbolNode)
        WHERE r.type IN $calls_type_ids
          AND r.source_start_line IN $source_lines
          AND r.source_start_col <= $start_col AND r.source_end_col >= $end_col
        RETURN
          'ImportedSymbol' as target_type,
          CAST(target.id AS INT64) as target_id,
          COALESCE(target.name, '') as name,
          '' as fqn,
          target.file_path as path,
          CAST(target.start_line AS INT64) as start_line,
          CAST(target.end_line AS INT64) as end_line,
          CAST(r.source_start_col AS INT64) as rel_start_col,
          CAST(r.source_end_col AS INT64) as rel_end_col,
          CAST(r.type AS INT64) as rel_type
        LIMIT 100
    "#;

    let mut hits = Vec::new();
    for q in [
        q_def_from_def,
        q_imp_from_def,
        q_def_from_file,
        q_imp_from_file,
    ] {
        let qr = conn
            .generic_query(q, base_params.clone())
            .map_err(|e| rmcp::ErrorData::new(ErrorCode::INVALID_REQUEST, e.to_string(), None))?;
        for row in qr.result.iter() {
            if row.len() < 10 {
                continue;
            }
            let raw = RawHit {
                target_type: row[0].to_string(),
                id: row[1].to_string(),
                name: row[2].to_string(),
                fqn: row[3].to_string(),
                path: row[4].to_string(),
                start_line_db: row[5].to_string().parse().unwrap_or(0),
                end_line_db: row[6].to_string().parse().unwrap_or(0),
                rel_start_col: row[7].to_string().parse().unwrap_or(0),
                rel_end_col: row[8].to_string().parse().unwrap_or(0),
                rel_type_id: row[9].to_string().parse().unwrap_or(0),
            };
            hits.push(raw);
        }
    }
    Ok(hits)
}
