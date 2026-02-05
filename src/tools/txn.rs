//! Transaction tools.
//!
//! Tools: strata_txn_begin, strata_txn_commit, strata_txn_rollback, strata_txn_info, strata_txn_active

use serde_json::{Map, Value as JsonValue};
use stratadb::{Command, TxnOptions};

use crate::convert::{get_optional_bool, output_to_json};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all transaction tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_txn_begin",
            "Begin a new transaction on the current branch. Operations within the transaction are atomic.",
            schema!(object {
                optional: { "read_only": boolean }
            }),
        ),
        ToolDef::new(
            "strata_txn_commit",
            "Commit the current transaction, making all changes permanent.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_txn_rollback",
            "Rollback the current transaction, discarding all changes.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_txn_info",
            "Get information about the current transaction. Returns null if no transaction is active.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_txn_active",
            "Check if a transaction is currently active. Returns true/false.",
            schema!(object {}),
        ),
    ]
}

/// Dispatch a transaction tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_txn_begin" => {
            let read_only = get_optional_bool(&args, "read_only").unwrap_or(false);

            let cmd = Command::TxnBegin {
                branch: session.branch_id(),
                options: Some(TxnOptions { read_only }),
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        "strata_txn_commit" => {
            let output = session.execute(Command::TxnCommit)?;
            Ok(output_to_json(output))
        }

        "strata_txn_rollback" => {
            let output = session.execute(Command::TxnRollback)?;
            Ok(output_to_json(output))
        }

        "strata_txn_info" => {
            let output = session.execute(Command::TxnInfo)?;
            Ok(output_to_json(output))
        }

        "strata_txn_active" => {
            let output = session.execute(Command::TxnIsActive)?;
            Ok(output_to_json(output))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
