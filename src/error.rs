//! Error types for the MCP server.
//!
//! Maps stratadb errors to MCP-friendly error responses.

use serde::{Deserialize, Serialize};
use stratadb::Error as StrataError;

/// MCP server errors.
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum McpError {
    /// Error from the underlying Strata database.
    #[error("strata error: {message}")]
    Strata {
        /// The error code from strata
        code: String,
        /// Human-readable error message
        message: String,
    },

    /// Unknown tool requested.
    #[error("unknown tool: {0}")]
    UnknownTool(String),

    /// Missing required argument.
    #[error("missing required argument: {0}")]
    MissingArg(String),

    /// Invalid argument value.
    #[error("invalid argument '{name}': {reason}")]
    InvalidArg {
        /// Argument name
        name: String,
        /// Reason why it's invalid
        reason: String,
    },

    /// Branch not found.
    #[error("branch not found: {0}")]
    BranchNotFound(String),

    /// JSON-RPC protocol error.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<StrataError> for McpError {
    fn from(err: StrataError) -> Self {
        let code = match &err {
            StrataError::KeyNotFound { .. } => "KEY_NOT_FOUND",
            StrataError::BranchNotFound { .. } => "BRANCH_NOT_FOUND",
            StrataError::CollectionNotFound { .. } => "COLLECTION_NOT_FOUND",
            StrataError::StreamNotFound { .. } => "STREAM_NOT_FOUND",
            StrataError::CellNotFound { .. } => "CELL_NOT_FOUND",
            StrataError::DocumentNotFound { .. } => "DOCUMENT_NOT_FOUND",
            StrataError::WrongType { .. } => "WRONG_TYPE",
            StrataError::InvalidKey { .. } => "INVALID_KEY",
            StrataError::InvalidPath { .. } => "INVALID_PATH",
            StrataError::InvalidInput { .. } => "INVALID_INPUT",
            StrataError::VersionConflict { .. } => "VERSION_CONFLICT",
            StrataError::TransitionFailed { .. } => "TRANSITION_FAILED",
            StrataError::Conflict { .. } => "CONFLICT",
            StrataError::BranchClosed { .. } => "BRANCH_CLOSED",
            StrataError::BranchExists { .. } => "BRANCH_EXISTS",
            StrataError::CollectionExists { .. } => "COLLECTION_EXISTS",
            StrataError::DimensionMismatch { .. } => "DIMENSION_MISMATCH",
            StrataError::ConstraintViolation { .. } => "CONSTRAINT_VIOLATION",
            StrataError::HistoryTrimmed { .. } => "HISTORY_TRIMMED",
            StrataError::Overflow { .. } => "OVERFLOW",
            StrataError::AccessDenied { .. } => "ACCESS_DENIED",
            StrataError::TransactionNotActive => "TXN_NOT_ACTIVE",
            StrataError::TransactionAlreadyActive => "TXN_ALREADY_ACTIVE",
            StrataError::TransactionConflict { .. } => "TXN_CONFLICT",
            StrataError::Io { .. } => "IO_ERROR",
            StrataError::Serialization { .. } => "SERIALIZATION_ERROR",
            StrataError::Internal { .. } => "INTERNAL_ERROR",
            StrataError::NotImplemented { .. } => "NOT_IMPLEMENTED",
        };

        McpError::Strata {
            code: code.to_string(),
            message: err.to_string(),
        }
    }
}

impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        McpError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::Protocol(format!("JSON error: {}", err))
    }
}

/// JSON-RPC error codes.
pub mod rpc_codes {
    /// Parse error - Invalid JSON was received.
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid Request - The JSON sent is not a valid Request object.
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found - The method does not exist / is not available.
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params - Invalid method parameter(s).
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error - Internal JSON-RPC error.
    pub const INTERNAL_ERROR: i32 = -32603;
}

impl McpError {
    /// Convert to JSON-RPC error code.
    pub fn rpc_code(&self) -> i32 {
        match self {
            McpError::UnknownTool(_) => rpc_codes::METHOD_NOT_FOUND,
            McpError::MissingArg(_) | McpError::InvalidArg { .. } => rpc_codes::INVALID_PARAMS,
            McpError::Protocol(_) => rpc_codes::INVALID_REQUEST,
            McpError::Strata { code, .. } => {
                // Map strata errors to appropriate RPC codes
                match code.as_str() {
                    "KEY_NOT_FOUND" | "BRANCH_NOT_FOUND" | "COLLECTION_NOT_FOUND"
                    | "CELL_NOT_FOUND" | "DOCUMENT_NOT_FOUND" | "STREAM_NOT_FOUND" => {
                        rpc_codes::INVALID_PARAMS
                    }
                    "INVALID_KEY" | "INVALID_PATH" | "INVALID_INPUT" | "WRONG_TYPE" => {
                        rpc_codes::INVALID_PARAMS
                    }
                    _ => rpc_codes::INTERNAL_ERROR,
                }
            }
            _ => rpc_codes::INTERNAL_ERROR,
        }
    }
}

/// Result type for MCP operations.
pub type Result<T> = std::result::Result<T, McpError>;
