//! Event log tools.
//!
//! Tools: strata_event_append, strata_event_get, strata_event_list, strata_event_len

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{
    get_optional_u64, get_string_arg, get_u64_arg, get_value_arg, output_to_json,
};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all event tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_event_append",
            "Append an event to the log. Returns the sequence number (version).",
            schema!(object {
                required: { "event_type": string, "payload": any }
            }),
        ),
        ToolDef::new(
            "strata_event_get",
            "Get an event by its sequence number. Returns null if not found.",
            schema!(object {
                required: { "sequence": integer }
            }),
        ),
        ToolDef::new(
            "strata_event_list",
            "List events of a specific type with optional pagination.",
            schema!(object {
                required: { "event_type": string },
                optional: { "limit": integer, "after_sequence": integer }
            }),
        ),
        ToolDef::new(
            "strata_event_len",
            "Get the total count of events in the log.",
            schema!(object {}),
        ),
    ]
}

/// Dispatch an event tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_event_append" => {
            let event_type = get_string_arg(&args, "event_type")?;
            let payload = get_value_arg(&args, "payload")?;

            let cmd = Command::EventAppend {
                branch: session.branch_id(),
                space: session.space_id(),
                event_type,
                payload,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_event_get" => {
            let sequence = get_u64_arg(&args, "sequence")?;

            let cmd = Command::EventGet {
                branch: session.branch_id(),
                space: session.space_id(),
                sequence,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_event_list" => {
            let event_type = get_string_arg(&args, "event_type")?;
            let limit = get_optional_u64(&args, "limit");
            let after_sequence = get_optional_u64(&args, "after_sequence");

            let cmd = Command::EventGetByType {
                branch: session.branch_id(),
                space: session.space_id(),
                event_type,
                limit,
                after_sequence,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_event_len" => {
            let cmd = Command::EventLen {
                branch: session.branch_id(),
                space: session.space_id(),
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
