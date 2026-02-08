//! Vector store tools.
//!
//! Tools: strata_vector_upsert, strata_vector_get, strata_vector_delete, strata_vector_search,
//!        strata_vector_create_collection, strata_vector_delete_collection,
//!        strata_vector_list_collections, strata_vector_stats, strata_vector_batch_upsert

use serde_json::{Map, Value as JsonValue};
use stratadb::{BatchVectorEntry, Command, DistanceMetric, FilterOp, MetadataFilter};

use crate::convert::{
    get_optional_string, get_optional_u64, get_string_arg, get_u64_arg, get_value_arg,
    get_vector_arg, json_to_value, output_to_json,
};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all vector tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_vector_upsert",
            "Insert or update a vector with optional metadata. Returns the version number.",
            schema!(object {
                required: { "collection": string, "key": string, "vector": array_number },
                optional: { "metadata": any }
            }),
        ),
        ToolDef::new(
            "strata_vector_get",
            "Get a vector by key. Returns the embedding, metadata, and version info. \
             Pass as_of (microsecond timestamp) for time-travel reads.",
            schema!(object {
                required: { "collection": string, "key": string },
                optional: { "as_of": integer }
            }),
        ),
        ToolDef::new(
            "strata_vector_delete",
            "Delete a vector. Returns true if the vector existed.",
            schema!(object {
                required: { "collection": string, "key": string }
            }),
        ),
        ToolDef::new(
            "strata_vector_search",
            "Search for similar vectors. Returns top-k matches with scores. \
             Filters narrow results by metadata: each filter has field (metadata key), \
             op (eq|ne|gt|gte|lt|lte|in|contains), and value. \
             Pass as_of (microsecond timestamp) for time-travel reads.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "collection": {"type": "string"},
                    "query": {"type": "array", "items": {"type": "number"}},
                    "k": {"type": "integer"},
                    "filter": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "field": {"type": "string", "description": "Metadata field name"},
                                "op": {
                                    "type": "string",
                                    "enum": ["eq", "ne", "gt", "gte", "lt", "lte", "in", "contains"],
                                    "description": "Comparison operator"
                                },
                                "value": {"description": "Value to compare against"}
                            },
                            "required": ["field", "op", "value"]
                        }
                    },
                    "metric": {"type": "string", "enum": ["cosine", "euclidean", "dot_product"]},
                    "as_of": {"type": "integer", "description": "Microsecond timestamp for time-travel reads"}
                },
                "required": ["collection", "query", "k"]
            }),
        ),
        ToolDef::new(
            "strata_vector_create_collection",
            "Create a new vector collection with specified dimension and distance metric.",
            schema!(object {
                required: { "collection": string, "dimension": integer },
                optional: { "metric": string }
            }),
        ),
        ToolDef::new(
            "strata_vector_delete_collection",
            "Delete a vector collection and all its vectors. Returns true if collection existed.",
            schema!(object {
                required: { "collection": string }
            }),
        ),
        ToolDef::new(
            "strata_vector_list_collections",
            "List all vector collections in the current branch/space.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_vector_stats",
            "Get detailed statistics for a specific collection.",
            schema!(object {
                required: { "collection": string }
            }),
        ),
        ToolDef::new(
            "strata_vector_batch_upsert",
            "Insert or update multiple vectors in a single operation. Returns version numbers.",
            schema!(object {
                required: { "collection": string, "entries": array_object }
            }),
        ),
    ]
}

/// Parse a distance metric from a string.
fn parse_metric(s: Option<&str>) -> Result<DistanceMetric> {
    match s {
        Some("cosine") | None => Ok(DistanceMetric::Cosine),
        Some("euclidean") => Ok(DistanceMetric::Euclidean),
        Some("dot_product") | Some("dotproduct") => Ok(DistanceMetric::DotProduct),
        Some(other) => Err(McpError::InvalidArg {
            name: "metric".to_string(),
            reason: format!(
                "Unknown metric '{}'. Use 'cosine', 'euclidean', or 'dot_product'.",
                other
            ),
        }),
    }
}

/// Parse a filter operation from a string.
fn parse_filter_op(s: &str) -> Result<FilterOp> {
    match s {
        "eq" | "=" | "==" => Ok(FilterOp::Eq),
        "ne" | "!=" | "<>" => Ok(FilterOp::Ne),
        "gt" | ">" => Ok(FilterOp::Gt),
        "gte" | ">=" => Ok(FilterOp::Gte),
        "lt" | "<" => Ok(FilterOp::Lt),
        "lte" | "<=" => Ok(FilterOp::Lte),
        "in" => Ok(FilterOp::In),
        "contains" => Ok(FilterOp::Contains),
        other => Err(McpError::InvalidArg {
            name: "filter.op".to_string(),
            reason: format!("Unknown filter operation '{}'", other),
        }),
    }
}

/// Parse filters from JSON array.
fn parse_filters(args: &Map<String, JsonValue>) -> Result<Option<Vec<MetadataFilter>>> {
    let arr = match args.get("filter") {
        Some(JsonValue::Array(a)) => a,
        Some(JsonValue::Null) | None => return Ok(None),
        _ => {
            return Err(McpError::InvalidArg {
                name: "filter".to_string(),
                reason: "Expected array of filter objects".to_string(),
            })
        }
    };

    if arr.is_empty() {
        return Ok(None);
    }

    let mut filters = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| McpError::InvalidArg {
            name: format!("filter[{}]", i),
            reason: "Expected filter object with field, op, and value".to_string(),
        })?;

        let field = obj
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArg {
                name: format!("filter[{}].field", i),
                reason: "Missing or invalid field".to_string(),
            })?
            .to_string();

        let op_str = obj
            .get("op")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArg {
                name: format!("filter[{}].op", i),
                reason: "Missing or invalid op".to_string(),
            })?;
        let op = parse_filter_op(op_str)?;

        let value_json = obj.get("value").cloned().ok_or_else(|| McpError::InvalidArg {
            name: format!("filter[{}].value", i),
            reason: "Missing value".to_string(),
        })?;
        let value = json_to_value(value_json)?;

        filters.push(MetadataFilter { field, op, value });
    }

    Ok(Some(filters))
}

/// Parse batch entries from JSON array.
fn parse_batch_entries(args: &Map<String, JsonValue>) -> Result<Vec<BatchVectorEntry>> {
    let arr = args
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| McpError::MissingArg("entries".to_string()))?;

    let mut entries = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| McpError::InvalidArg {
            name: format!("entries[{}]", i),
            reason: "Expected object with key, vector, and optional metadata".to_string(),
        })?;

        let key = obj
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidArg {
                name: format!("entries[{}].key", i),
                reason: "Missing or invalid key".to_string(),
            })?
            .to_string();

        let vector_arr = obj.get("vector").and_then(|v| v.as_array()).ok_or_else(|| {
            McpError::InvalidArg {
                name: format!("entries[{}].vector", i),
                reason: "Missing or invalid vector".to_string(),
            }
        })?;

        let vector: Result<Vec<f32>> = vector_arr
            .iter()
            .enumerate()
            .map(|(j, v)| {
                v.as_f64().map(|f| f as f32).ok_or_else(|| McpError::InvalidArg {
                    name: format!("entries[{}].vector[{}]", i, j),
                    reason: "Expected number".to_string(),
                })
            })
            .collect();
        let vector = vector?;

        let metadata = match obj.get("metadata") {
            Some(JsonValue::Null) | None => None,
            Some(v) => Some(json_to_value(v.clone())?),
        };

        entries.push(BatchVectorEntry {
            key,
            vector,
            metadata,
        });
    }

    Ok(entries)
}

/// Dispatch a vector tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_vector_upsert" => {
            let collection = get_string_arg(&args, "collection")?;
            let key = get_string_arg(&args, "key")?;
            let vector = get_vector_arg(&args, "vector")?;
            let metadata = match args.get("metadata") {
                Some(JsonValue::Null) | None => None,
                Some(_) => Some(get_value_arg(&args, "metadata")?),
            };

            let cmd = Command::VectorUpsert {
                branch: session.branch_id(),
                space: session.space_id(),
                collection,
                key,
                vector,
                metadata,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_vector_get" => {
            let collection = get_string_arg(&args, "collection")?;
            let key = get_string_arg(&args, "key")?;
            let as_of = get_optional_u64(&args, "as_of");

            let cmd = Command::VectorGet {
                branch: session.branch_id(),
                space: session.space_id(),
                collection,
                key,
                as_of,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_vector_delete" => {
            let collection = get_string_arg(&args, "collection")?;
            let key = get_string_arg(&args, "key")?;

            let cmd = Command::VectorDelete {
                branch: session.branch_id(),
                space: session.space_id(),
                collection,
                key,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_vector_search" => {
            let collection = get_string_arg(&args, "collection")?;
            let query = get_vector_arg(&args, "query")?;
            let k = get_u64_arg(&args, "k")?;
            let filter = parse_filters(&args)?;
            let metric = parse_metric(get_optional_string(&args, "metric").as_deref())?;
            let as_of = get_optional_u64(&args, "as_of");

            let cmd = Command::VectorSearch {
                branch: session.branch_id(),
                space: session.space_id(),
                collection,
                query,
                k,
                filter,
                metric: Some(metric),
                as_of,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_vector_create_collection" => {
            let collection = get_string_arg(&args, "collection")?;
            let dimension = get_u64_arg(&args, "dimension")?;
            let metric = parse_metric(get_optional_string(&args, "metric").as_deref())?;

            let cmd = Command::VectorCreateCollection {
                branch: session.branch_id(),
                space: session.space_id(),
                collection,
                dimension,
                metric,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_vector_delete_collection" => {
            let collection = get_string_arg(&args, "collection")?;

            let cmd = Command::VectorDeleteCollection {
                branch: session.branch_id(),
                space: session.space_id(),
                collection,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_vector_list_collections" => {
            let cmd = Command::VectorListCollections {
                branch: session.branch_id(),
                space: session.space_id(),
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_vector_stats" => {
            let collection = get_string_arg(&args, "collection")?;

            let cmd = Command::VectorCollectionStats {
                branch: session.branch_id(),
                space: session.space_id(),
                collection,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_vector_batch_upsert" => {
            let collection = get_string_arg(&args, "collection")?;
            let entries = parse_batch_entries(&args)?;

            let cmd = Command::VectorBatchUpsert {
                branch: session.branch_id(),
                space: session.space_id(),
                collection,
                entries,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
