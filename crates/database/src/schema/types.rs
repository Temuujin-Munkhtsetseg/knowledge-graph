use arrow::{
    array::{Array, Int32Array, Int64Array, StringArray, UInt8Array, UInt32Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use std::sync::Arc;

/// Trait for extracting field values from a node with proper type handling
pub trait NodeFieldAccess {
    /// Extract a string field value
    fn get_string_field(&self, field_name: &str) -> Option<String> {
        let _ = field_name;
        None
    }

    /// Extract an i32 field value
    fn get_i32_field(&self, field_name: &str) -> Option<i32> {
        let _ = field_name;
        None
    }

    /// Extract an i64 field value
    fn get_i64_field(&self, field_name: &str) -> Option<i64> {
        let _ = field_name;
        None
    }

    /// Extract a u32 field value
    fn get_u32_field(&self, field_name: &str) -> Option<u32> {
        let _ = field_name;
        None
    }

    /// Extract a u8 field value
    fn get_u8_field(&self, field_name: &str) -> Option<u8> {
        let _ = field_name;
        None
    }

    /// Extract an ID field value as u32 using a callback
    fn get_id_field<F>(&self, field_name: &str, id_callback: F) -> Option<u32>
    where
        F: FnOnce(&Self) -> u32,
    {
        let _ = field_name;
        Some(id_callback(self))
    }
}

/// Trait for converting a slice of nodes to an Arrow RecordBatch
pub trait ToArrowBatch<T>
where
    T: NodeFieldAccess,
{
    /// Convert a slice of nodes to a RecordBatch using the provided schema and ID callback
    fn to_record_batch<F>(
        nodes: &[T],
        table: &NodeTable,
        id_callback: F,
    ) -> Result<RecordBatch, Box<dyn std::error::Error>>
    where
        F: Fn(&T) -> u32 + Clone,
    {
        let mut arrays: Vec<Arc<dyn Array>> = Vec::new();

        for column in &table.columns {
            let array: Arc<dyn Array> = match column.data_type {
                KuzuDataType::UInt32 => {
                    // Check if this is the primary key field - use ID callback, otherwise treat as regular field
                    if column.name == table.primary_key {
                        let callback = id_callback.clone();
                        let values: Vec<u32> = nodes
                            .iter()
                            .map(|node| {
                                node.get_id_field(&column.name, |n| callback(n))
                                    .unwrap_or(0)
                            })
                            .collect();
                        Arc::new(UInt32Array::from(values))
                    } else {
                        let values: Vec<u32> = nodes
                            .iter()
                            .map(|node| node.get_u32_field(&column.name).unwrap_or(0))
                            .collect();
                        Arc::new(UInt32Array::from(values))
                    }
                }
                KuzuDataType::String => {
                    let values: Vec<String> = nodes
                        .iter()
                        .map(|node| node.get_string_field(&column.name).unwrap_or_default())
                        .collect();
                    Arc::new(StringArray::from(values))
                }
                KuzuDataType::Int32 => {
                    let values: Vec<i32> = nodes
                        .iter()
                        .map(|node| node.get_i32_field(&column.name).unwrap_or(0))
                        .collect();
                    Arc::new(Int32Array::from(values))
                }
                KuzuDataType::Int64 => {
                    let values: Vec<i64> = nodes
                        .iter()
                        .map(|node| node.get_i64_field(&column.name).unwrap_or(0))
                        .collect();
                    Arc::new(Int64Array::from(values))
                }
                KuzuDataType::UInt8 => {
                    let values: Vec<u8> = nodes
                        .iter()
                        .map(|node| node.get_u8_field(&column.name).unwrap_or(0))
                        .collect();
                    Arc::new(UInt8Array::from(values))
                }
                _ => return Err(format!("Unsupported data type: {:?}", column.data_type).into()),
            };
            arrays.push(array);
        }

        let record_batch = RecordBatch::try_new(table.to_arrow_schema(), arrays)?;
        Ok(record_batch)
    }
}

/// Generic converter that implements ToArrowBatch for any node type
pub struct ArrowBatchConverter;

impl<T> ToArrowBatch<T> for ArrowBatchConverter
where
    T: NodeFieldAccess,
{
    // Uses the default implementation
}

/// Represents a Kuzu node table definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeTable {
    pub name: &'static str,
    pub parquet_filename: &'static str,
    pub columns: Vec<ColumnDefinition>,
    pub primary_key: &'static str,
}

impl NodeTable {
    pub fn get_parquet_filename(&self) -> String {
        format!("{}_table.parquet", self.name.to_lowercase())
    }

    pub fn to_arrow_schema(&self) -> Arc<Schema> {
        let fields: Vec<Field> = self
            .columns
            .iter()
            .map(|col| Field::new(&col.name, col.data_type.clone().into(), col.nullable))
            .collect();
        let schema = Schema::new(fields);
        Arc::new(schema)
    }
}

// TODO: We're gonna use macros to generate code for querying nodes
// E.g table-level actions like loads will derive FROM the table

// TODO: We'll also just want a node definition... to derive the table from

/// Represents a Kuzu relationship table definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipTable {
    pub name: &'static str,
    pub columns: Vec<ColumnDefinition>,
    pub from_to_pairs: Vec<(&'static NodeTable, &'static NodeTable)>,
}

impl RelationshipTable {
    pub fn get_parquet_filenames_from_pairs(&self) -> Vec<((&NodeTable, &NodeTable), String)> {
        self.from_to_pairs
            .iter()
            .map(|(from, to)| {
                (
                    (*from, *to),
                    format!(
                        "{}_to_{}_relationships.parquet",
                        from.name.to_lowercase(),
                        to.name.to_lowercase()
                    ),
                )
            })
            .collect()
    }
}

// TODO: Same thing for the relationship table

/// Represents a column definition in a table
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: KuzuDataType,
    pub is_primary_key: bool,
    pub nullable: bool,
}

macro_rules! generate_data_type_methods {
    ($($method_name:ident => $variant:ident),* $(,)?) => {
        $(
            pub fn $method_name(mut self) -> Self {
                self.data_type = KuzuDataType::$variant;
                self
            }
        )*
    };
}

impl ColumnDefinition {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: KuzuDataType::String,
            is_primary_key: false,
            nullable: false,
        }
    }

    // generates methods for each data type e.g string(), int32(), etc.
    generate_data_type_methods! {
        string => String,
        int32 => Int32,
        int64 => Int64,
        uint32 => UInt32,
        uint8 => UInt8,
        float => Float,
        double => Double,
        boolean => Boolean,
        date => Date,
        timestamp => Timestamp,
        string_array => StringArray,
        int64_array => Int64Array,
    }

    pub fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self
    }

    pub fn nullable(mut self) -> Self {
        self.nullable = true;
        self
    }
}

/// Kuzu data types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KuzuDataType {
    String,
    Int32,
    Int64,
    UInt32,
    UInt8,
    Float,
    Double,
    Boolean,
    Date,
    Timestamp,
    StringArray,
    Int64Array,
}

/// Convert Kuzu data type to Arrow data type
impl From<KuzuDataType> for DataType {
    fn from(data_type: KuzuDataType) -> Self {
        match data_type {
            KuzuDataType::String => DataType::Utf8,
            KuzuDataType::Int32 => DataType::Int32,
            KuzuDataType::Int64 => DataType::Int64,
            KuzuDataType::UInt32 => DataType::UInt32,
            KuzuDataType::UInt8 => DataType::UInt8,
            KuzuDataType::Float => DataType::Float32,
            KuzuDataType::Double => DataType::Float64,
            KuzuDataType::Boolean => DataType::Boolean,
            KuzuDataType::Date => DataType::Date32,
            _ => panic!("Unsupported data type: {}", data_type),
        }
    }
}

impl std::fmt::Display for KuzuDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KuzuDataType::String => write!(f, "STRING"),
            KuzuDataType::Int32 => write!(f, "INT32"),
            KuzuDataType::Int64 => write!(f, "INT64"),
            KuzuDataType::UInt32 => write!(f, "UINT32"),
            KuzuDataType::UInt8 => write!(f, "UINT8"),
            KuzuDataType::Float => write!(f, "FLOAT"),
            KuzuDataType::Double => write!(f, "DOUBLE"),
            KuzuDataType::Boolean => write!(f, "BOOLEAN"),
            KuzuDataType::Date => write!(f, "DATE"),
            KuzuDataType::Timestamp => write!(f, "TIMESTAMP"),
            KuzuDataType::StringArray => write!(f, "STRING[]"),
            KuzuDataType::Int64Array => write!(f, "INT64[]"),
        }
    }
}

/// Schema statistics
#[derive(Debug, Clone)]
pub struct SchemaStats {
    pub total_tables: usize,
    pub node_tables: usize,
    pub relationship_tables: usize,
    pub total_nodes: usize,
    pub total_relationships: usize,
    pub table_names: Vec<String>,
}

impl std::fmt::Display for SchemaStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Schema Stats: {} tables ({} node, {} rel), {} nodes, {} relationships\nTables: {}",
            self.total_tables,
            self.node_tables,
            self.relationship_tables,
            self.total_nodes,
            self.total_relationships,
            self.table_names.join(", ")
        )
    }
}
