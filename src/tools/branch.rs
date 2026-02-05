//! Branch management tools.
//!
//! Tools: strata_branch_create, strata_branch_get, strata_branch_list, strata_branch_exists,
//!        strata_branch_delete, strata_branch_fork, strata_branch_diff, strata_branch_merge,
//!        strata_branch_switch

use serde_json::{Map, Value as JsonValue};
use stratadb::{BranchId, Command, MergeStrategy};

use crate::convert::{get_optional_string, get_optional_u64, get_string_arg, output_to_json};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all branch tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_branch_create",
            "Create a new empty branch. Optionally specify branch_id (UUID or name).",
            schema!(object {
                optional: { "branch_id": string }
            }),
        ),
        ToolDef::new(
            "strata_branch_get",
            "Get information about a specific branch.",
            schema!(object {
                required: { "branch": string }
            }),
        ),
        ToolDef::new(
            "strata_branch_list",
            "List all branches with optional filtering and pagination.",
            schema!(object {
                optional: { "limit": integer, "offset": integer }
            }),
        ),
        ToolDef::new(
            "strata_branch_exists",
            "Check if a branch exists. Returns true/false.",
            schema!(object {
                required: { "branch": string }
            }),
        ),
        ToolDef::new(
            "strata_branch_delete",
            "Delete a branch and all its data. Cannot delete the 'default' branch.",
            schema!(object {
                required: { "branch": string }
            }),
        ),
        ToolDef::new(
            "strata_branch_fork",
            "Fork the current branch to a new branch, copying all data.",
            schema!(object {
                required: { "destination": string }
            }),
        ),
        ToolDef::new(
            "strata_branch_diff",
            "Compare two branches and return their differences.",
            schema!(object {
                required: { "branch_a": string, "branch_b": string }
            }),
        ),
        ToolDef::new(
            "strata_branch_merge",
            "Merge a source branch into the current branch. Strategy: 'last_writer_wins' or 'strict'.",
            schema!(object {
                required: { "source": string },
                optional: { "strategy": string }
            }),
        ),
        ToolDef::new(
            "strata_branch_switch",
            "Switch the session's current branch context. All subsequent operations will use this branch.",
            schema!(object {
                required: { "branch": string }
            }),
        ),
    ]
}

/// Dispatch a branch tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_branch_create" => {
            let branch_id = get_optional_string(&args, "branch_id");

            let cmd = Command::BranchCreate {
                branch_id,
                metadata: None,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_branch_get" => {
            let branch = get_string_arg(&args, "branch")?;

            let cmd = Command::BranchGet {
                branch: BranchId::from(branch),
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_branch_list" => {
            let limit = get_optional_u64(&args, "limit");
            let offset = get_optional_u64(&args, "offset");

            let cmd = Command::BranchList {
                state: None,
                limit,
                offset,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_branch_exists" => {
            let branch = get_string_arg(&args, "branch")?;

            let cmd = Command::BranchExists {
                branch: BranchId::from(branch),
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_branch_delete" => {
            let branch = get_string_arg(&args, "branch")?;

            let cmd = Command::BranchDelete {
                branch: BranchId::from(branch),
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_branch_fork" => {
            let destination = get_string_arg(&args, "destination")?;

            let info = session.fork_branch(&destination)?;
            Ok(serde_json::json!({
                "source": info.source,
                "destination": info.destination,
                "keys_copied": info.keys_copied,
            }))
        }

        "strata_branch_diff" => {
            let branch_a = get_string_arg(&args, "branch_a")?;
            let branch_b = get_string_arg(&args, "branch_b")?;

            let diff = session.diff_branches(&branch_a, &branch_b)?;

            // Convert SpaceDiff entries to JSON (manually serialize BranchDiffEntry)
            let spaces: Vec<JsonValue> = diff
                .spaces
                .into_iter()
                .map(|s| {
                    let added: Vec<JsonValue> = s.added.into_iter().map(|e| serde_json::json!({
                        "key": e.key,
                        "primitive": format!("{:?}", e.primitive),
                        "space": e.space,
                        "value_a": e.value_a,
                        "value_b": e.value_b,
                    })).collect();
                    let removed: Vec<JsonValue> = s.removed.into_iter().map(|e| serde_json::json!({
                        "key": e.key,
                        "primitive": format!("{:?}", e.primitive),
                        "space": e.space,
                        "value_a": e.value_a,
                        "value_b": e.value_b,
                    })).collect();
                    let modified: Vec<JsonValue> = s.modified.into_iter().map(|e| serde_json::json!({
                        "key": e.key,
                        "primitive": format!("{:?}", e.primitive),
                        "space": e.space,
                        "value_a": e.value_a,
                        "value_b": e.value_b,
                    })).collect();
                    serde_json::json!({
                        "space": s.space,
                        "added": added,
                        "removed": removed,
                        "modified": modified,
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "branch_a": diff.branch_a,
                "branch_b": diff.branch_b,
                "summary": {
                    "total_added": diff.summary.total_added,
                    "total_removed": diff.summary.total_removed,
                    "total_modified": diff.summary.total_modified,
                },
                "spaces": spaces,
            }))
        }

        "strata_branch_merge" => {
            let source = get_string_arg(&args, "source")?;
            let strategy_str = get_optional_string(&args, "strategy");

            let strategy = match strategy_str.as_deref() {
                Some("strict") => MergeStrategy::Strict,
                Some("last_writer_wins") | None => MergeStrategy::LastWriterWins,
                Some(other) => {
                    return Err(McpError::InvalidArg {
                        name: "strategy".to_string(),
                        reason: format!(
                            "Unknown merge strategy '{}'. Use 'last_writer_wins' or 'strict'.",
                            other
                        ),
                    });
                }
            };

            let info = session.merge_branch(&source, strategy)?;

            // Convert conflicts to JSON
            let conflicts: Vec<JsonValue> = info
                .conflicts
                .into_iter()
                .map(|c| {
                    serde_json::json!({
                        "key": c.key,
                        "primitive": format!("{:?}", c.primitive),
                        "space": c.space,
                        "source_value": c.source_value,
                        "target_value": c.target_value,
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "keys_applied": info.keys_applied,
                "spaces_merged": info.spaces_merged,
                "conflicts": conflicts,
            }))
        }

        "strata_branch_switch" => {
            let branch = get_string_arg(&args, "branch")?;
            session.switch_branch(&branch)?;
            Ok(serde_json::json!({
                "switched": true,
                "branch": branch
            }))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
