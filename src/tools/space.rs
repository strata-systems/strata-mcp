//! Space management tools.
//!
//! Tools: strata_space_list, strata_space_create, strata_space_delete, strata_space_switch

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{get_optional_bool, get_string_arg, output_to_json};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all space tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_space_list",
            "List all spaces in the current branch.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_space_create",
            "Create a new space explicitly.",
            schema!(object {
                required: { "space": string }
            }),
        ),
        ToolDef::new(
            "strata_space_delete",
            "Delete a space. Must be empty unless force=true.",
            schema!(object {
                required: { "space": string },
                optional: { "force": boolean }
            }),
        ),
        ToolDef::new(
            "strata_space_switch",
            "Switch the session's current space context. All subsequent operations will use this space.",
            schema!(object {
                required: { "space": string }
            }),
        ),
    ]
}

/// Dispatch a space tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_space_list" => {
            let cmd = Command::SpaceList {
                branch: session.branch_id(),
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_space_create" => {
            let space = get_string_arg(&args, "space")?;

            let cmd = Command::SpaceCreate {
                branch: session.branch_id(),
                space,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_space_delete" => {
            let space = get_string_arg(&args, "space")?;
            let force = get_optional_bool(&args, "force").unwrap_or(false);

            let cmd = Command::SpaceDelete {
                branch: session.branch_id(),
                space,
                force,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_space_switch" => {
            let space = get_string_arg(&args, "space")?;
            session.switch_space(&space);
            Ok(serde_json::json!({
                "switched": true,
                "space": space
            }))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
