use crate::graph::RelationshipTypeMapping;
use crate::querying::QueryResultRow;
use anyhow::Error;

pub type QueryResultMapper = fn(&dyn QueryResultRow, usize) -> Result<serde_json::Value, Error>;

pub const STRING_MAPPER: QueryResultMapper = |row: &dyn QueryResultRow, index: usize| {
    Ok(serde_json::Value::String(row.get_string_value(index)?))
};

pub const INT_MAPPER: QueryResultMapper = |row: &dyn QueryResultRow, index: usize| {
    Ok(serde_json::Value::Number(row.get_int_value(index)?.into()))
};

pub const RELATIONSHIP_TYPE_MAPPER: QueryResultMapper = |row: &dyn QueryResultRow, index: usize| {
    let value: u8 = row.get_uint_value(index)?.try_into()?;

    let relationship_type_mapper = RelationshipTypeMapping::new();
    let type_name = relationship_type_mapper.get_type_name(value);

    Ok(serde_json::Value::String(type_name.to_string()))
};
