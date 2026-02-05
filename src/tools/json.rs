//! JSON document store tools.
//!
//! Tools: strata_json_set, strata_json_get, strata_json_delete, strata_json_list, strata_json_history

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{
    get_optional_string, get_optional_u64, get_string_arg, get_value_arg, output_to_json,
};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all JSON tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_json_set",
            "Set a value at a JSONPath in a document. Creates the document if it doesn't exist. Returns version number.",
            schema!(object {
                required: { "key": string, "path": string, "value": any }
            }),
        ),
        ToolDef::new(
            "strata_json_get",
            "Get a value at a JSONPath from a document. Use '$' for the entire document. Returns null if not found.",
            schema!(object {
                required: { "key": string, "path": string }
            }),
        ),
        ToolDef::new(
            "strata_json_delete",
            "Delete a JSON document. Returns the count of elements removed (0 or 1).",
            schema!(object {
                required: { "key": string, "path": string }
            }),
        ),
        ToolDef::new(
            "strata_json_list",
            "List JSON document keys with optional prefix filter and cursor-based pagination.",
            schema!(object {
                optional: { "prefix": string, "cursor": string, "limit": integer }
            }),
        ),
        ToolDef::new(
            "strata_json_history",
            "Get the full version history for a JSON document.",
            schema!(object {
                required: { "key": string }
            }),
        ),
    ]
}

/// Dispatch a JSON tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_json_set" => {
            let key = get_string_arg(&args, "key")?;
            let path = get_string_arg(&args, "path")?;
            let value = get_value_arg(&args, "value")?;

            let cmd = Command::JsonSet {
                branch: session.branch_id(),
                space: session.space_id(),
                key,
                path,
                value,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_json_get" => {
            let key = get_string_arg(&args, "key")?;
            let path = get_string_arg(&args, "path")?;

            let cmd = Command::JsonGet {
                branch: session.branch_id(),
                space: session.space_id(),
                key,
                path,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_json_delete" => {
            let key = get_string_arg(&args, "key")?;
            let path = get_string_arg(&args, "path")?;

            let cmd = Command::JsonDelete {
                branch: session.branch_id(),
                space: session.space_id(),
                key,
                path,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_json_list" => {
            let prefix = get_optional_string(&args, "prefix");
            let cursor = get_optional_string(&args, "cursor");
            let limit = get_optional_u64(&args, "limit").unwrap_or(100);

            let cmd = Command::JsonList {
                branch: session.branch_id(),
                space: session.space_id(),
                prefix,
                cursor,
                limit,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_json_history" => {
            let key = get_string_arg(&args, "key")?;

            let cmd = Command::JsonGetv {
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
