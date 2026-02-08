//! Model configuration tools.
//!
//! Tools: strata_configure_model

use serde_json::{Map, Value as JsonValue};
use stratadb::Command;

use crate::convert::{get_optional_string, get_optional_u64, get_string_arg};
use crate::error::{McpError, Result};
use crate::schema;
use crate::session::McpSession;
use crate::tools::ToolDef;

/// Get all configuration tool definitions.
pub fn tools() -> Vec<ToolDef> {
    vec![ToolDef::new(
        "strata_configure_model",
        "Configure an inference model endpoint for intelligent search. \
         When configured, search() transparently expands queries using the model \
         for better recall. Accepts any OpenAI-compatible endpoint (Ollama, vLLM, OpenAI).",
        schema!(object {
            required: { "endpoint": string, "model": string },
            optional: { "api_key": string, "timeout_ms": integer }
        }),
    )]
}

/// Dispatch a configuration tool call.
pub fn dispatch(
    session: &mut McpSession,
    name: &str,
    args: Map<String, JsonValue>,
) -> Result<JsonValue> {
    match name {
        "strata_configure_model" => {
            let endpoint = get_string_arg(&args, "endpoint")?;
            let model = get_string_arg(&args, "model")?;
            let api_key = get_optional_string(&args, "api_key");
            let timeout_ms = get_optional_u64(&args, "timeout_ms");

            let cmd = Command::ConfigureModel {
                endpoint,
                model,
                api_key,
                timeout_ms,
            };
            session.execute(cmd)?;
            Ok(serde_json::json!({ "status": "ok" }))
        }

        _ => Err(McpError::UnknownTool(name.to_string())),
    }
}
