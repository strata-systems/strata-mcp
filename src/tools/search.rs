//! Cross-primitive search tools.
//!
//! Tools: strata_search

use serde_json::{Map, Value as JsonValue};
use stratadb::{Command, SearchQuery, TimeRangeInput};

use crate::convert::{
    get_optional_bool, get_optional_string, get_optional_u64, get_string_arg, output_to_json,
};
use crate::error::{McpError, Result};
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all search tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef::new(
        "strata_search",
        "Search across multiple primitives (kv, json, state, event) for matching content. \
         Returns ranked results with scores and snippets. Use this to find data when you \
         don't know which primitive contains it.",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "k": { "type": "integer" },
                "primitives": { "type": "array", "items": { "type": "string" } },
                "time_range": {
                    "type": "object",
                    "properties": {
                        "start": { "type": "string" },
                        "end": { "type": "string" }
                    },
                    "required": ["start", "end"]
                },
                "mode": { "type": "string", "enum": ["keyword", "hybrid"] },
                "expand": { "type": "boolean" },
                "rerank": { "type": "boolean" }
            },
            "required": ["query"]
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
            let time_range = get_optional_time_range(&args);
            let mode = get_optional_string(&args, "mode");
            let expand = get_optional_bool(&args, "expand");
            let rerank = get_optional_bool(&args, "rerank");

            let sq = SearchQuery {
                query,
                k,
                primitives,
                time_range,
                mode,
                expand,
                rerank,
            };

            let cmd = Command::Search {
                branch: session.branch_id(),
                space: session.space_id(),
                search: sq,
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

/// Helper to extract an optional time_range object with start/end strings.
fn get_optional_time_range(args: &Map<String, JsonValue>) -> Option<TimeRangeInput> {
    let obj = args.get("time_range")?.as_object()?;
    let start = obj.get("start")?.as_str()?.to_string();
    let end = obj.get("end")?.as_str()?.to_string();
    Some(TimeRangeInput { start, end })
}
