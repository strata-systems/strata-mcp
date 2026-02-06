//! Key-value store tools.
//!
//! Tools: strata_kv_put, strata_kv_get, strata_kv_delete, strata_kv_list, strata_kv_history,
//!        strata_kv_put_many, strata_kv_get_many, strata_kv_delete_many

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{
    get_optional_string, get_optional_u64, get_string_arg, get_value_arg, json_to_value,
    output_to_json,
};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all KV tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_kv_put",
            "Store a key-value pair in the current branch/space. Values can be any JSON type. \
             Returns the new version number. Use strata_kv_put_many for multiple keys.",
            schema!(object {
                required: { "key": string, "value": any }
            }),
        ),
        ToolDef::new(
            "strata_kv_get",
            "Get the value for a key with version info. Returns null if key doesn't exist. \
             Use strata_kv_get_many to fetch multiple keys in one call.",
            schema!(object {
                required: { "key": string }
            }),
        ),
        ToolDef::new(
            "strata_kv_delete",
            "Delete a key from the current branch/space. Returns true if the key existed. \
             Use strata_kv_delete_many for multiple keys.",
            schema!(object {
                required: { "key": string }
            }),
        ),
        ToolDef::new(
            "strata_kv_list",
            "List keys with optional prefix filter. Returns array of key names. \
             Use cursor and limit for pagination through large result sets.",
            schema!(object {
                optional: { "prefix": string, "cursor": string, "limit": integer }
            }),
        ),
        ToolDef::new(
            "strata_kv_history",
            "Get all historical versions of a key. Returns array of {value, version, timestamp}. \
             Useful for auditing changes or implementing undo.",
            schema!(object {
                required: { "key": string }
            }),
        ),
        ToolDef::new(
            "strata_kv_put_many",
            "Store multiple key-value pairs in a single operation. More efficient than \
             multiple strata_kv_put calls. Returns array of version numbers.",
            schema!(object {
                required: { "items": array_object }
            }),
        ),
        ToolDef::new(
            "strata_kv_get_many",
            "Get multiple keys in a single operation. More efficient than multiple \
             strata_kv_get calls. Returns array of values (null for missing keys).",
            schema!(object {
                required: { "keys": array_string }
            }),
        ),
        ToolDef::new(
            "strata_kv_delete_many",
            "Delete multiple keys in a single operation. More efficient than multiple \
             strata_kv_delete calls. Returns array of booleans (true if key existed).",
            schema!(object {
                required: { "keys": array_string }
            }),
        ),
    ]
}

/// Dispatch a KV tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_kv_put" => {
            let key = get_string_arg(&args, "key")?;
            let value = get_value_arg(&args, "value")?;

            let cmd = Command::KvPut {
                branch: session.branch_id(),
                space: session.space_id(),
                key,
                value,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_kv_get" => {
            let key = get_string_arg(&args, "key")?;

            let cmd = Command::KvGet {
                branch: session.branch_id(),
                space: session.space_id(),
                key,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_kv_delete" => {
            let key = get_string_arg(&args, "key")?;

            let cmd = Command::KvDelete {
                branch: session.branch_id(),
                space: session.space_id(),
                key,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_kv_list" => {
            let prefix = get_optional_string(&args, "prefix");
            let cursor = get_optional_string(&args, "cursor");
            let limit = get_optional_u64(&args, "limit");

            let cmd = Command::KvList {
                branch: session.branch_id(),
                space: session.space_id(),
                prefix,
                cursor,
                limit,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_kv_history" => {
            let key = get_string_arg(&args, "key")?;

            let cmd = Command::KvGetv {
                branch: session.branch_id(),
                space: session.space_id(),
                key,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_kv_put_many" => {
            let items = args
                .get("items")
                .and_then(|v| v.as_array())
                .ok_or_else(|| McpError::MissingArg("items".to_string()))?;

            let mut versions = Vec::new();
            for item in items {
                let key = item
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::InvalidArg {
                        name: "items".to_string(),
                        reason: "Each item must have a 'key' string field".to_string(),
                    })?
                    .to_string();

                let value_json = item.get("value").cloned().ok_or_else(|| McpError::InvalidArg {
                    name: "items".to_string(),
                    reason: "Each item must have a 'value' field".to_string(),
                })?;
                let value = json_to_value(value_json)?;

                let cmd = Command::KvPut {
                    branch: session.branch_id(),
                    space: session.space_id(),
                    key,
                    value,
                };
                let output = session.execute(cmd)?;
                versions.push(output_to_json(output));
            }
            Ok(JsonValue::Array(versions))
        }

        "strata_kv_get_many" => {
            let keys = args
                .get("keys")
                .and_then(|v| v.as_array())
                .ok_or_else(|| McpError::MissingArg("keys".to_string()))?;

            let mut results = Vec::new();
            for key_value in keys {
                let key = key_value
                    .as_str()
                    .ok_or_else(|| McpError::InvalidArg {
                        name: "keys".to_string(),
                        reason: "Keys must be strings".to_string(),
                    })?
                    .to_string();

                let cmd = Command::KvGet {
                    branch: session.branch_id(),
                    space: session.space_id(),
                    key,
                };
                let output = session.execute(cmd)?;
                results.push(output_to_json(output));
            }
            Ok(JsonValue::Array(results))
        }

        "strata_kv_delete_many" => {
            let keys = args
                .get("keys")
                .and_then(|v| v.as_array())
                .ok_or_else(|| McpError::MissingArg("keys".to_string()))?;

            let mut results = Vec::new();
            for key_value in keys {
                let key = key_value
                    .as_str()
                    .ok_or_else(|| McpError::InvalidArg {
                        name: "keys".to_string(),
                        reason: "Keys must be strings".to_string(),
                    })?
                    .to_string();

                let cmd = Command::KvDelete {
                    branch: session.branch_id(),
                    space: session.space_id(),
                    key,
                };
                let output = session.execute(cmd)?;
                results.push(output_to_json(output));
            }
            Ok(JsonValue::Array(results))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
