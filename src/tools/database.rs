//! Database-level tools.
//!
//! Tools: strata_db_ping, strata_db_info, strata_db_flush, strata_db_compact

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::output_to_json;
use crate::error::Result;
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all database tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "strata_db_ping",
            "Ping the database to check connectivity and get version info. \
             Use this as a health check before starting work.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_db_info",
            "Get database statistics including version, uptime in seconds, branch count, \
             and total keys. Useful for monitoring and capacity planning.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_db_flush",
            "Force pending writes to disk immediately. Normally writes are buffered \
             for performance; use this before critical operations or shutdown.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_db_compact",
            "Trigger storage compaction to reclaim disk space from deleted data. \
             This is done automatically but can be triggered manually if needed.",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_db_time_range",
            "Get the available time range for the current branch. Returns oldest_ts and latest_ts \
             (microsecond timestamps) for use with as_of time-travel reads. Returns null timestamps \
             if the branch has no data.",
            schema!(object {}),
        ),
    ]
}

/// Dispatch a database tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    _args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    let cmd = match name {
        "strata_db_ping" => Command::Ping,
        "strata_db_info" => Command::Info,
        "strata_db_flush" => Command::Flush,
        "strata_db_compact" => Command::Compact,
        "strata_db_time_range" => Command::TimeRange {
            branch: session.branch_id(),
        },
        _ => return Err(crate::error::McpError::UnknownTool(name.to_string())),
    };

    let output = session.execute(cmd)?;
    Ok(output_to_json(output))
}
