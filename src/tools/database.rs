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
            "Ping the database to check connectivity",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_db_info",
            "Get database information including version, uptime, and statistics",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_db_flush",
            "Flush pending writes to disk for durability",
            schema!(object {}),
        ),
        ToolDef::new(
            "strata_db_compact",
            "Trigger storage compaction to reclaim space",
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
        _ => return Err(crate::error::McpError::UnknownTool(name.to_string())),
    };

    let output = session.execute(cmd)?;
    Ok(output_to_json(output))
}
