//! Cross-primitive search tools.
//!
//! Tools: strata_search

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{get_optional_u64, get_string_arg, output_to_json};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all search tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef::new(
        "strata_search",
        "Search across multiple primitives (kv, json, state, event) for matching content. \
         Returns ranked results with scores and snippets. Use this to find data when you \
         don't know which primitive contains it.",
        schema!(object {
            required: { "query": string },
            optional: { "k": integer, "primitives": array_string }
        }),
    )]
}

/// Dispatch a search tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_search" => {
            let query = get_string_arg(&args, "query")?;
            let k = get_optional_u64(&args, "k");
            let primitives = get_optional_string_array(&args, "primitives");

            let cmd = Command::Search {
                branch: session.branch_id(),
                space: session.space_id(),
                query,
                k,
                primitives,
            };
            let output = session.execute(cmd)?;
            Ok(output_to_json(output))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}

/// Helper to get an optional array of strings.
fn get_optional_string_array(args: &Map<String, JsonValue>, name: &str) -> Option<Vec<String>> {
    args.get(name).and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    })
}
