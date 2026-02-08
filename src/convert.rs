//! Conversion utilities between JSON and stratadb types.
//!
//! Provides bidirectional conversion between serde_json::Value and stratadb::Value,
//! as well as Output to JSON conversion for MCP responses.

use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use stratadb::{Output, Value, VersionedValue};

use crate::error::{McpError, Result};

/// Convert a JSON value to a stratadb Value.
pub fn json_to_value(json: JsonValue) -> Result<Value> {
    match json {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(b) => Ok(Value::Bool(b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Err(McpError::InvalidArg {
                    name: "value".to_string(),
                    reason: "Number out of range".to_string(),
                })
            }
        }
        JsonValue::String(s) => Ok(Value::String(s)),
        JsonValue::Array(arr) => {
            let values: Result<Vec<Value>> = arr.into_iter().map(json_to_value).collect();
            Ok(Value::Array(values?))
        }
        JsonValue::Object(map) => {
            let mut obj = HashMap::new();
            for (k, v) in map {
                obj.insert(k, json_to_value(v)?);
            }
            Ok(Value::Object(obj))
        }
    }
}

/// Convert a stratadb Value to a JSON value.
/// Uses stratadb's built-in conversion which handles base64 encoding for bytes.
pub fn value_to_json(value: Value) -> JsonValue {
    // stratadb::Value implements Into<serde_json::Value>
    value.into()
}

/// Convert a VersionedValue to JSON.
pub fn versioned_to_json(vv: VersionedValue) -> JsonValue {
    serde_json::json!({
        "value": value_to_json(vv.value),
        "version": vv.version,
        "timestamp": vv.timestamp,
    })
}

/// Convert an Output to JSON for MCP response.
pub fn output_to_json(output: Output) -> JsonValue {
    match output {
        Output::Unit => JsonValue::Null,
        Output::Maybe(opt) => opt.map_or(JsonValue::Null, value_to_json),
        Output::MaybeVersioned(opt) => opt.map_or(JsonValue::Null, versioned_to_json),
        Output::MaybeVersion(opt) => opt.map_or(JsonValue::Null, |v| JsonValue::Number(v.into())),
        Output::Version(v) => serde_json::json!({ "version": v }),
        Output::Bool(b) => JsonValue::Bool(b),
        Output::Uint(n) => JsonValue::Number(n.into()),

        Output::VersionedValues(values) => {
            JsonValue::Array(values.into_iter().map(versioned_to_json).collect())
        }
        Output::VersionHistory(opt) => opt.map_or(JsonValue::Null, |values| {
            JsonValue::Array(values.into_iter().map(versioned_to_json).collect())
        }),
        Output::Keys(keys) => JsonValue::Array(keys.into_iter().map(JsonValue::String).collect()),

        Output::JsonListResult { keys, cursor } => {
            let mut obj = Map::new();
            obj.insert(
                "keys".to_string(),
                JsonValue::Array(keys.into_iter().map(JsonValue::String).collect()),
            );
            if let Some(c) = cursor {
                obj.insert("cursor".to_string(), JsonValue::String(c));
            }
            JsonValue::Object(obj)
        }

        Output::VectorMatches(matches) => {
            let arr: Vec<JsonValue> = matches
                .into_iter()
                .map(|m| {
                    serde_json::json!({
                        "key": m.key,
                        "score": m.score,
                        "metadata": m.metadata.map(value_to_json),
                    })
                })
                .collect();
            JsonValue::Array(arr)
        }

        Output::VectorData(opt) => opt.map_or(JsonValue::Null, |vd| {
            serde_json::json!({
                "key": vd.key,
                "embedding": vd.data.embedding,
                "metadata": vd.data.metadata.map(value_to_json),
                "version": vd.version,
                "timestamp": vd.timestamp,
            })
        }),

        Output::VectorCollectionList(collections) => {
            let arr: Vec<JsonValue> = collections
                .into_iter()
                .map(|c| {
                    serde_json::json!({
                        "name": c.name,
                        "dimension": c.dimension,
                        "metric": format!("{:?}", c.metric).to_lowercase(),
                        "count": c.count,
                        "index_type": c.index_type,
                        "memory_bytes": c.memory_bytes,
                    })
                })
                .collect();
            JsonValue::Array(arr)
        }

        Output::Versions(versions) => {
            JsonValue::Array(versions.into_iter().map(|v| v.into()).collect())
        }

        Output::MaybeBranchInfo(opt) => opt.map_or(JsonValue::Null, |bi| {
            serde_json::json!({
                "id": bi.info.id.as_str(),
                "status": format!("{:?}", bi.info.status).to_lowercase(),
                "created_at": bi.info.created_at,
                "updated_at": bi.info.updated_at,
                "parent_id": bi.info.parent_id.map(|p| p.as_str().to_string()),
                "version": bi.version,
                "timestamp": bi.timestamp,
            })
        }),

        Output::BranchInfoList(branches) => {
            let arr: Vec<JsonValue> = branches
                .into_iter()
                .map(|bi| {
                    serde_json::json!({
                        "id": bi.info.id.as_str(),
                        "status": format!("{:?}", bi.info.status).to_lowercase(),
                        "created_at": bi.info.created_at,
                        "updated_at": bi.info.updated_at,
                        "parent_id": bi.info.parent_id.map(|p| p.as_str().to_string()),
                        "version": bi.version,
                        "timestamp": bi.timestamp,
                    })
                })
                .collect();
            JsonValue::Array(arr)
        }

        Output::BranchWithVersion { info, version } => {
            serde_json::json!({
                "id": info.id.as_str(),
                "status": format!("{:?}", info.status).to_lowercase(),
                "created_at": info.created_at,
                "updated_at": info.updated_at,
                "parent_id": info.parent_id.map(|p| p.as_str().to_string()),
                "version": version,
            })
        }

        Output::TxnInfo(opt) => opt.map_or(JsonValue::Null, |ti| {
            serde_json::json!({
                "id": ti.id,
                "status": format!("{:?}", ti.status).to_lowercase(),
                "started_at": ti.started_at,
            })
        }),

        Output::TxnBegun => serde_json::json!({ "status": "begun" }),
        Output::TxnCommitted { version } => {
            serde_json::json!({ "status": "committed", "version": version })
        }
        Output::TxnAborted => serde_json::json!({ "status": "aborted" }),

        Output::DatabaseInfo(info) => {
            serde_json::json!({
                "version": info.version,
                "uptime_secs": info.uptime_secs,
                "branch_count": info.branch_count,
                "total_keys": info.total_keys,
            })
        }

        Output::Pong { version } => serde_json::json!({ "pong": true, "version": version }),

        Output::SearchResults(results) => {
            let arr: Vec<JsonValue> = results
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "entity": r.entity,
                        "primitive": r.primitive,
                        "score": r.score,
                        "rank": r.rank,
                        "snippet": r.snippet,
                    })
                })
                .collect();
            JsonValue::Array(arr)
        }

        Output::SpaceList(spaces) => {
            JsonValue::Array(spaces.into_iter().map(JsonValue::String).collect())
        }

        Output::BranchExported(result) => {
            serde_json::json!({
                "branch_id": result.branch_id,
                "path": result.path,
                "entry_count": result.entry_count,
                "bundle_size": result.bundle_size,
            })
        }

        Output::BranchImported(result) => {
            serde_json::json!({
                "branch_id": result.branch_id,
                "transactions_applied": result.transactions_applied,
                "keys_written": result.keys_written,
            })
        }

        Output::BundleValidated(result) => {
            serde_json::json!({
                "branch_id": result.branch_id,
                "format_version": result.format_version,
                "entry_count": result.entry_count,
                "checksums_valid": result.checksums_valid,
            })
        }

        Output::TimeRange { oldest_ts, latest_ts } => {
            serde_json::json!({
                "oldest_ts": oldest_ts,
                "latest_ts": latest_ts,
            })
        }
    }
}

/// Helper to get a required string argument from JSON arguments.
pub fn get_string_arg(args: &Map<String, JsonValue>, name: &str) -> Result<String> {
    args.get(name)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| McpError::MissingArg(name.to_string()))
}

/// Helper to get an optional string argument from JSON arguments.
pub fn get_optional_string(args: &Map<String, JsonValue>, name: &str) -> Option<String> {
    args.get(name).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Helper to get a required u64 argument from JSON arguments.
pub fn get_u64_arg(args: &Map<String, JsonValue>, name: &str) -> Result<u64> {
    args.get(name)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| McpError::MissingArg(name.to_string()))
}

/// Helper to get an optional u64 argument from JSON arguments.
pub fn get_optional_u64(args: &Map<String, JsonValue>, name: &str) -> Option<u64> {
    args.get(name).and_then(|v| v.as_u64())
}

/// Helper to get a required value argument and convert it to stratadb Value.
pub fn get_value_arg(args: &Map<String, JsonValue>, name: &str) -> Result<Value> {
    let json = args
        .get(name)
        .cloned()
        .ok_or_else(|| McpError::MissingArg(name.to_string()))?;
    json_to_value(json)
}

/// Helper to get a required f32 vector argument.
pub fn get_vector_arg(args: &Map<String, JsonValue>, name: &str) -> Result<Vec<f32>> {
    let arr = args
        .get(name)
        .and_then(|v| v.as_array())
        .ok_or_else(|| McpError::MissingArg(name.to_string()))?;

    arr.iter()
        .map(|v| {
            v.as_f64().map(|f| f as f32).ok_or_else(|| McpError::InvalidArg {
                name: name.to_string(),
                reason: "Expected array of numbers".to_string(),
            })
        })
        .collect()
}

/// Helper to get an optional boolean argument.
pub fn get_optional_bool(args: &Map<String, JsonValue>, name: &str) -> Option<bool> {
    args.get(name).and_then(|v| v.as_bool())
}
