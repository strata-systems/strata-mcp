//! MCP server implementation.
//!
//! Handles JSON-RPC 2.0 over stdio according to the MCP protocol specification.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as JsonValue};
use std::io::{BufRead, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::error::{rpc_codes, McpError, Result};
use crate::session::McpSession;
use crate::tools::ToolRegistry;

/// MCP protocol version we support.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server information.
const SERVER_NAME: &str = "strata-mcp";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<JsonValue>,
    pub method: String,
    #[serde(default)]
    pub params: Option<JsonValue>,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

impl JsonRpcResponse {
    /// Create a success response.
    pub fn success(id: Option<JsonValue>, result: JsonValue) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Option<JsonValue>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }

    /// Create an error response from an McpError.
    pub fn from_error(id: Option<JsonValue>, err: McpError) -> Self {
        Self::error(id, err.rpc_code(), err.to_string())
    }
}

/// MCP server.
pub struct McpServer {
    session: McpSession,
    registry: ToolRegistry,
    initialized: bool,
}

impl McpServer {
    /// Create a new MCP server with the given session.
    pub fn new(session: McpSession) -> Self {
        Self {
            session,
            registry: ToolRegistry::new(),
            initialized: false,
        }
    }

    /// Run the server, reading from stdin and writing to stdout.
    pub async fn run(&mut self) -> Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line).await?;

            if bytes_read == 0 {
                // EOF - client disconnected
                break;
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse the request
            let response = match serde_json::from_str::<JsonRpcRequest>(line) {
                Ok(request) => self.handle_request(request),
                Err(e) => JsonRpcResponse::error(
                    None,
                    rpc_codes::PARSE_ERROR,
                    format!("Parse error: {}", e),
                ),
            };

            // Send response
            let response_json = serde_json::to_string(&response)?;
            stdout.write_all(response_json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    /// Run the server synchronously (for non-tokio environments).
    pub fn run_sync(&mut self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let mut line = String::new();

        let stdin_lock = stdin.lock();
        let mut reader = std::io::BufReader::new(stdin_lock);

        loop {
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;

            if bytes_read == 0 {
                // EOF - client disconnected
                break;
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse the request
            let response = match serde_json::from_str::<JsonRpcRequest>(line) {
                Ok(request) => self.handle_request(request),
                Err(e) => JsonRpcResponse::error(
                    None,
                    rpc_codes::PARSE_ERROR,
                    format!("Parse error: {}", e),
                ),
            };

            // Send response
            let response_json = serde_json::to_string(&response)?;
            writeln!(stdout, "{}", response_json)?;
            stdout.flush()?;
        }

        Ok(())
    }

    /// Handle a single JSON-RPC request.
    fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return JsonRpcResponse::error(
                request.id,
                rpc_codes::INVALID_REQUEST,
                "Invalid JSON-RPC version".to_string(),
            );
        }

        // Route to appropriate handler
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request),
            "initialized" => {
                // Client acknowledgment - no response needed for notifications
                // but we'll still respond with null to be safe
                JsonRpcResponse::success(request.id, JsonValue::Null)
            }
            "tools/list" => self.handle_tools_list(request),
            "tools/call" => self.handle_tools_call(request),
            "ping" => JsonRpcResponse::success(request.id, serde_json::json!({})),
            _ => JsonRpcResponse::error(
                request.id,
                rpc_codes::METHOD_NOT_FOUND,
                format!("Unknown method: {}", request.method),
            ),
        }
    }

    /// Handle the initialize request.
    fn handle_initialize(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;

        JsonRpcResponse::success(
            request.id,
            serde_json::json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }),
        )
    }

    /// Handle the tools/list request.
    fn handle_tools_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tools: Vec<JsonValue> = self
            .registry
            .tools()
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema
                })
            })
            .collect();

        JsonRpcResponse::success(request.id, serde_json::json!({ "tools": tools }))
    }

    /// Handle the tools/call request.
    fn handle_tools_call(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Extract name and arguments from params
        let params = match &request.params {
            Some(JsonValue::Object(obj)) => obj,
            _ => {
                return JsonRpcResponse::error(
                    request.id,
                    rpc_codes::INVALID_PARAMS,
                    "Missing params object".to_string(),
                )
            }
        };

        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => {
                return JsonRpcResponse::error(
                    request.id,
                    rpc_codes::INVALID_PARAMS,
                    "Missing 'name' in params".to_string(),
                )
            }
        };

        let arguments = match params.get("arguments") {
            Some(JsonValue::Object(obj)) => obj.clone(),
            Some(JsonValue::Null) | None => Map::new(),
            _ => {
                return JsonRpcResponse::error(
                    request.id,
                    rpc_codes::INVALID_PARAMS,
                    "'arguments' must be an object".to_string(),
                )
            }
        };

        // Dispatch the tool call
        match self.registry.dispatch(&mut self.session, &name, arguments) {
            Ok(result) => {
                // MCP tool responses are wrapped in content array
                JsonRpcResponse::success(
                    request.id,
                    serde_json::json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string(&result).unwrap_or_else(|_| "null".to_string())
                        }]
                    }),
                )
            }
            Err(err) => JsonRpcResponse::from_error(request.id, err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_response_success() {
        let response = JsonRpcResponse::success(Some(JsonValue::Number(1.into())), serde_json::json!({"ok": true}));
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_json_rpc_response_error() {
        let response = JsonRpcResponse::error(Some(JsonValue::Number(1.into())), -32600, "Invalid".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\""));
        assert!(!json.contains("\"result\""));
    }
}
