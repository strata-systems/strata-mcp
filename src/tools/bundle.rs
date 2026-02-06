//! Branch bundle tools for data portability.
//!
//! Tools: strata_bundle_export, strata_bundle_import, strata_bundle_validate

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{get_string_arg, output_to_json};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all bundle tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_bundle_export",
            "Export a branch to a portable bundle file. The bundle contains all data \
             and can be imported into another database. Returns the file path and statistics.",
            schema!(object {
                required: { "branch": string, "path": string }
            }),
        ),
        ToolDef::new(
            "strata_bundle_import",
            "Import a branch from a bundle file. Creates a new branch with all the \
             data from the bundle. Returns the imported branch ID and statistics.",
            schema!(object {
                required: { "path": string }
            }),
        ),
        ToolDef::new(
            "strata_bundle_validate",
            "Validate a bundle file without importing it. Checks format version, \
             entry count, and checksums. Returns validation results.",
            schema!(object {
                required: { "path": string }
            }),
        ),
    ]
}

/// Dispatch a bundle tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_bundle_export" => {
            let branch_id = get_string_arg(&args, "branch")?;
            let path = get_string_arg(&args, "path")?;

            let cmd = Command::BranchExport { branch_id, path };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_bundle_import" => {
            let path = get_string_arg(&args, "path")?;

            let cmd = Command::BranchImport { path };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_bundle_validate" => {
            let path = get_string_arg(&args, "path")?;

            let cmd = Command::BranchBundleValidate { path };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
