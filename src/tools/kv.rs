//! Key-value store tools.
//!
//! Tools: strata_kv_put, strata_kv_get, strata_kv_delete, strata_kv_list, strata_kv_history

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{
    get_optional_string, get_optional_u64, get_string_arg, get_value_arg, output_to_json,
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
            "Store a key-value pair. Returns the version number.",
            schema!(object {
                required: { "key": string, "value": any }
            }),
        ),
        ToolDef::new(
            "strata_kv_get",
            "Get the value for a key. Returns null if key doesn't exist.",
            schema!(object {
                required: { "key": string }
            }),
        ),
        ToolDef::new(
            "strata_kv_delete",
            "Delete a key. Returns true if the key existed.",
            schema!(object {
                required: { "key": string }
            }),
        ),
        ToolDef::new(
            "strata_kv_list",
            "List keys with optional prefix filter. Supports cursor-based pagination.",
            schema!(object {
                optional: { "prefix": string, "cursor": string, "limit": integer }
            }),
        ),
        ToolDef::new(
            "strata_kv_history",
            "Get the full version history for a key.",
            schema!(object {
                required: { "key": string }
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

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
