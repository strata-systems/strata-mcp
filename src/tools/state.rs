//! State cell tools.
//!
//! Tools: strata_state_set, strata_state_get, strata_state_delete, strata_state_init,
//!        strata_state_cas, strata_state_list, strata_state_history

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{
    get_optional_string, get_optional_u64, get_string_arg, get_value_arg, output_to_json,
};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all state tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_state_set",
            "Set a state cell value (unconditional write). Returns the version number.",
            schema!(object {
                required: { "cell": string, "value": any }
            }),
        ),
        ToolDef::new(
            "strata_state_get",
            "Get the current value of a state cell. Returns null if cell doesn't exist. \
             Pass as_of (microsecond timestamp) for time-travel reads.",
            schema!(object {
                required: { "cell": string },
                optional: { "as_of": integer }
            }),
        ),
        ToolDef::new(
            "strata_state_delete",
            "Delete a state cell. Returns true if the cell existed.",
            schema!(object {
                required: { "cell": string }
            }),
        ),
        ToolDef::new(
            "strata_state_init",
            "Initialize a state cell only if it doesn't exist. Returns the version number.",
            schema!(object {
                required: { "cell": string, "value": any }
            }),
        ),
        ToolDef::new(
            "strata_state_cas",
            "Compare-and-swap: update cell only if expected_counter matches. Returns new version or null if CAS failed.",
            schema!(object {
                required: { "cell": string, "value": any },
                optional: { "expected_counter": integer }
            }),
        ),
        ToolDef::new(
            "strata_state_list",
            "List state cell names with optional prefix filter. \
             Pass as_of (microsecond timestamp) for time-travel reads.",
            schema!(object {
                optional: { "prefix": string, "as_of": integer }
            }),
        ),
        ToolDef::new(
            "strata_state_history",
            "Get the full version history for a state cell. \
             Pass as_of (microsecond timestamp) to get history up to that point.",
            schema!(object {
                required: { "cell": string },
                optional: { "as_of": integer }
            }),
        ),
    ]
}

/// Dispatch a state tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_state_set" => {
            let cell = get_string_arg(&args, "cell")?;
            let value = get_value_arg(&args, "value")?;

            let cmd = Command::StateSet {
                branch: session.branch_id(),
                space: session.space_id(),
                cell,
                value,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_state_get" => {
            let cell = get_string_arg(&args, "cell")?;
            let as_of = get_optional_u64(&args, "as_of");

            let cmd = Command::StateGet {
                branch: session.branch_id(),
                space: session.space_id(),
                cell,
                as_of,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_state_delete" => {
            let cell = get_string_arg(&args, "cell")?;

            let cmd = Command::StateDelete {
                branch: session.branch_id(),
                space: session.space_id(),
                cell,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_state_init" => {
            let cell = get_string_arg(&args, "cell")?;
            let value = get_value_arg(&args, "value")?;

            let cmd = Command::StateInit {
                branch: session.branch_id(),
                space: session.space_id(),
                cell,
                value,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_state_cas" => {
            let cell = get_string_arg(&args, "cell")?;
            let value = get_value_arg(&args, "value")?;
            let expected_counter = get_optional_u64(&args, "expected_counter");

            let cmd = Command::StateCas {
                branch: session.branch_id(),
                space: session.space_id(),
                cell,
                expected_counter,
                value,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_state_list" => {
            let prefix = get_optional_string(&args, "prefix");
            let as_of = get_optional_u64(&args, "as_of");

            let cmd = Command::StateList {
                branch: session.branch_id(),
                space: session.space_id(),
                prefix,
                as_of,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_state_history" => {
            let cell = get_string_arg(&args, "cell")?;
            let as_of = get_optional_u64(&args, "as_of");

            let cmd = Command::StateGetv {
                branch: session.branch_id(),
                space: session.space_id(),
                cell,
                as_of,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
