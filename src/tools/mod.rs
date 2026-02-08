//! Tool registry and category definitions.
//!
//! Provides the infrastructure for registering and dispatching MCP tools.

pub mod branch;
pub mod bundle;
pub mod config;
pub mod database;
pub mod event;
pub mod json;
pub mod kv;
pub mod retention;
pub mod search;
pub mod space;
pub mod state;
pub mod txn;
pub mod vector;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};

use crate::error::{McpError, Result};
use crate::session::McpSession;

/// A tool definition for the MCP tools/list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// Tool name (e.g., "strata_kv_put")
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for the input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: JsonValue,
}

impl ToolDef {
    /// Create a new tool definition.
    pub fn new(name: &str, description: &str, input_schema: JsonValue) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            input_schema,
        }
    }
}

/// Registry of all available tools.
pub struct ToolRegistry {
    tools: Vec<ToolDef>,
}

impl ToolRegistry {
    /// Create a new registry with all tools registered.
    pub fn new() -> Self {
        let mut tools = Vec::new();

        // Register all tool categories
        tools.extend(database::tools());
        tools.extend(kv::tools());
        tools.extend(state::tools());
        tools.extend(event::tools());
        tools.extend(json::tools());
        tools.extend(space::tools());
        tools.extend(branch::tools());
        tools.extend(vector::tools());
        tools.extend(txn::tools());
        tools.extend(search::tools());
        tools.extend(bundle::tools());
        tools.extend(retention::tools());
        tools.extend(config::tools());

        Self { tools }
    }

    /// Get all tool definitions.
    pub fn tools(&self) -> &[ToolDef] {
        &self.tools
    }

    /// Dispatch a tool call to the appropriate handler.
    pub fn dispatch(
        &self,
        session: &mut McpSession,
        name: &str,
        args: Map<String, JsonValue>,
    ) -> Result<JsonValue> {
        // Route based on prefix
        if name.starts_with("strata_db_") {
            database::dispatch(session, name, args)
        } else if name.starts_with("strata_kv_") {
            kv::dispatch(session, name, args)
        } else if name.starts_with("strata_state_") {
            state::dispatch(session, name, args)
        } else if name.starts_with("strata_event_") {
            event::dispatch(session, name, args)
        } else if name.starts_with("strata_json_") {
            json::dispatch(session, name, args)
        } else if name.starts_with("strata_space_") {
            space::dispatch(session, name, args)
        } else if name.starts_with("strata_branch_") {
            branch::dispatch(session, name, args)
        } else if name.starts_with("strata_vector_") {
            vector::dispatch(session, name, args)
        } else if name.starts_with("strata_txn_") {
            txn::dispatch(session, name, args)
        } else if name.starts_with("strata_search") {
            search::dispatch(session, name, args)
        } else if name.starts_with("strata_configure_") {
            config::dispatch(session, name, args)
        } else if name.starts_with("strata_bundle_") {
            bundle::dispatch(session, name, args)
        } else if name.starts_with("strata_retention_") {
            retention::dispatch(session, name, args)
        } else {
            Err(McpError::UnknownTool(name.to_string()))
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro for creating JSON Schema for tool input parameters.
#[macro_export]
macro_rules! schema {
    // Object with required and optional properties
    (object {
        required: { $($req_name:literal : $req_type:tt),* $(,)? },
        optional: { $($opt_name:literal : $opt_type:tt),* $(,)? }
    }) => {{
        let mut required = Vec::new();
        $(required.push($req_name);)*

        let mut props = serde_json::Map::new();
        $(props.insert($req_name.to_string(), schema!(@type $req_type));)*
        $(props.insert($opt_name.to_string(), schema!(@type $opt_type));)*

        serde_json::json!({
            "type": "object",
            "properties": props,
            "required": required
        })
    }};

    // Object with only required properties
    (object {
        required: { $($req_name:literal : $req_type:tt),* $(,)? }
    }) => {{
        let mut required = Vec::new();
        $(required.push($req_name);)*

        let mut props = serde_json::Map::new();
        $(props.insert($req_name.to_string(), schema!(@type $req_type));)*

        serde_json::json!({
            "type": "object",
            "properties": props,
            "required": required
        })
    }};

    // Object with only optional properties
    (object {
        optional: { $($opt_name:literal : $opt_type:tt),* $(,)? }
    }) => {{
        let mut props = serde_json::Map::new();
        $(props.insert($opt_name.to_string(), schema!(@type $opt_type));)*

        serde_json::json!({
            "type": "object",
            "properties": props,
            "required": []
        })
    }};

    // Empty object (no parameters)
    (object {}) => {{
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }};

    // Type mappings
    (@type string) => { serde_json::json!({"type": "string"}) };
    (@type number) => { serde_json::json!({"type": "number"}) };
    (@type integer) => { serde_json::json!({"type": "integer"}) };
    (@type boolean) => { serde_json::json!({"type": "boolean"}) };
    (@type any) => { serde_json::json!({}) };
    (@type array_number) => { serde_json::json!({"type": "array", "items": {"type": "number"}}) };
    (@type array_string) => { serde_json::json!({"type": "array", "items": {"type": "string"}}) };
    (@type array_object) => { serde_json::json!({"type": "array", "items": {"type": "object"}}) };
}
