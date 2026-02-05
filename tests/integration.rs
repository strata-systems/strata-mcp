//! Integration tests for the MCP server.

use serde_json::{json, Map, Value as JsonValue};
use strata_mcp::{McpSession, ToolRegistry};
use stratadb::Strata;

/// Create a test session with an in-memory database.
fn test_session() -> McpSession {
    let db = Strata::cache().expect("Failed to create cache database");
    McpSession::new(db)
}

/// Helper to dispatch a tool call.
fn call_tool(
    session: &mut McpSession,
    registry: &ToolRegistry,
    name: &str,
    args: JsonValue,
) -> JsonValue {
    let args_map: Map<String, JsonValue> = match args {
        JsonValue::Object(m) => m,
        _ => Map::new(),
    };
    registry.dispatch(session, name, args_map).expect(&format!("Tool {} failed", name))
}

/// Extract the "value" field from a versioned result, or return the value directly if not versioned.
fn extract_value(result: &JsonValue) -> &JsonValue {
    if let Some(val) = result.get("value") {
        val
    } else {
        result
    }
}

// =============================================================================
// Database Tools
// =============================================================================

#[test]
fn test_db_ping() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(&mut session, &registry, "strata_db_ping", json!({}));
    assert!(result.get("pong").is_some());
}

#[test]
fn test_db_info() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(&mut session, &registry, "strata_db_info", json!({}));
    assert!(result.get("version").is_some());
    assert!(result.get("branch_count").is_some());
}

// =============================================================================
// KV Tools
// =============================================================================

#[test]
fn test_kv_put_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Put a value
    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_put",
        json!({"key": "test_key", "value": "hello world"}),
    );
    assert!(result.get("version").is_some());

    // Get the value - may be versioned or raw depending on output type
    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_get",
        json!({"key": "test_key"}),
    );
    assert_eq!(extract_value(&result), &json!("hello world"));
}

#[test]
fn test_kv_delete() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Put a value
    call_tool(
        &mut session,
        &registry,
        "strata_kv_put",
        json!({"key": "to_delete", "value": 42}),
    );

    // Delete it
    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_delete",
        json!({"key": "to_delete"}),
    );
    assert_eq!(result, json!(true));

    // Verify it's gone
    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_get",
        json!({"key": "to_delete"}),
    );
    assert_eq!(result, json!(null));
}

#[test]
fn test_kv_list() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Put multiple values
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "user:1", "value": "alice"}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "user:2", "value": "bob"}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "item:1", "value": "book"}));

    // List all
    let result = call_tool(&mut session, &registry, "strata_kv_list", json!({}));
    let keys = result.as_array().expect("Expected array");
    assert_eq!(keys.len(), 3);

    // List with prefix
    let result = call_tool(&mut session, &registry, "strata_kv_list", json!({"prefix": "user:"}));
    let keys = result.as_array().expect("Expected array");
    assert_eq!(keys.len(), 2);
}

// =============================================================================
// State Tools
// =============================================================================

#[test]
fn test_state_set_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Set a state
    let result = call_tool(
        &mut session,
        &registry,
        "strata_state_set",
        json!({"cell": "counter", "value": 100}),
    );
    assert!(result.get("version").is_some());

    // Get the state - may be versioned
    let result = call_tool(
        &mut session,
        &registry,
        "strata_state_get",
        json!({"cell": "counter"}),
    );
    assert_eq!(extract_value(&result), &json!(100));
}

#[test]
fn test_state_init() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Init a cell
    call_tool(
        &mut session,
        &registry,
        "strata_state_init",
        json!({"cell": "status", "value": "pending"}),
    );

    // Get the value - may be versioned
    let result = call_tool(
        &mut session,
        &registry,
        "strata_state_get",
        json!({"cell": "status"}),
    );
    assert_eq!(extract_value(&result), &json!("pending"));
}

// =============================================================================
// Event Tools
// =============================================================================

#[test]
fn test_event_append_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Append an event
    let result = call_tool(
        &mut session,
        &registry,
        "strata_event_append",
        json!({"event_type": "user_action", "payload": {"action": "click", "target": "button"}}),
    );
    assert!(result.get("version").is_some());

    // Get event count
    let result = call_tool(&mut session, &registry, "strata_event_len", json!({}));
    assert_eq!(result, json!(1));
}

// =============================================================================
// JSON Tools
// =============================================================================

#[test]
fn test_json_set_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Set a JSON document
    call_tool(
        &mut session,
        &registry,
        "strata_json_set",
        json!({"key": "config", "path": "$", "value": {"theme": "dark", "lang": "en"}}),
    );

    // Get the whole document - may be versioned
    let result = call_tool(
        &mut session,
        &registry,
        "strata_json_get",
        json!({"key": "config", "path": "$"}),
    );
    let value = extract_value(&result);
    assert_eq!(value.get("theme").and_then(|v| v.as_str()), Some("dark"));

    // Get a specific path
    let result = call_tool(
        &mut session,
        &registry,
        "strata_json_get",
        json!({"key": "config", "path": "$.theme"}),
    );
    assert_eq!(extract_value(&result), &json!("dark"));
}

// =============================================================================
// Branch Tools
// =============================================================================

#[test]
fn test_branch_create_list() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Create a branch
    let result = call_tool(
        &mut session,
        &registry,
        "strata_branch_create",
        json!({"branch_id": "test-branch"}),
    );
    assert_eq!(result.get("id").and_then(|v| v.as_str()), Some("test-branch"));

    // List branches
    let result = call_tool(&mut session, &registry, "strata_branch_list", json!({}));
    let branches = result.as_array().expect("Expected array");
    assert!(branches.len() >= 2); // default + test-branch

    // Check exists
    let result = call_tool(
        &mut session,
        &registry,
        "strata_branch_exists",
        json!({"branch": "test-branch"}),
    );
    assert_eq!(result, json!(true));
}

#[test]
fn test_branch_switch() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Create a branch
    call_tool(&mut session, &registry, "strata_branch_create", json!({"branch_id": "feature"}));

    // Put data in default branch
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "x", "value": 1}));

    // Switch to feature branch
    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "feature"}));

    // Data should not exist in feature branch
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "x"}));
    assert_eq!(result, json!(null));

    // Put different data
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "x", "value": 2}));

    // Switch back to default
    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "default"}));

    // Original data should be there - may be versioned
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "x"}));
    assert_eq!(extract_value(&result), &json!(1));
}

#[test]
fn test_branch_fork() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Put data in default branch
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "shared", "value": "original"}));

    // Fork to a new branch
    let result = call_tool(&mut session, &registry, "strata_branch_fork", json!({"destination": "forked"}));
    assert!(result.get("keys_copied").is_some());

    // Switch to forked branch
    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "forked"}));

    // Data should be copied - may be versioned
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "shared"}));
    assert_eq!(extract_value(&result), &json!("original"));
}

// =============================================================================
// Space Tools
// =============================================================================

#[test]
fn test_space_operations() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Create a space
    call_tool(&mut session, &registry, "strata_space_create", json!({"space": "my-space"}));

    // List spaces
    let result = call_tool(&mut session, &registry, "strata_space_list", json!({}));
    let spaces = result.as_array().expect("Expected array");
    assert!(spaces.iter().any(|s| s.as_str() == Some("my-space")));

    // Switch to the space
    call_tool(&mut session, &registry, "strata_space_switch", json!({"space": "my-space"}));

    // Put data in the new space
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "space-key", "value": "space-value"}));

    // Switch back to default space
    call_tool(&mut session, &registry, "strata_space_switch", json!({"space": "default"}));

    // Data should not exist in default space
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "space-key"}));
    assert_eq!(result, json!(null));
}

// =============================================================================
// Vector Tools
// =============================================================================

#[test]
fn test_vector_operations() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Create a collection
    call_tool(
        &mut session,
        &registry,
        "strata_vector_create_collection",
        json!({"collection": "embeddings", "dimension": 4, "metric": "cosine"}),
    );

    // Upsert vectors
    call_tool(
        &mut session,
        &registry,
        "strata_vector_upsert",
        json!({"collection": "embeddings", "key": "v1", "vector": [1.0, 0.0, 0.0, 0.0]}),
    );
    call_tool(
        &mut session,
        &registry,
        "strata_vector_upsert",
        json!({"collection": "embeddings", "key": "v2", "vector": [0.0, 1.0, 0.0, 0.0]}),
    );

    // Search
    let result = call_tool(
        &mut session,
        &registry,
        "strata_vector_search",
        json!({"collection": "embeddings", "query": [1.0, 0.0, 0.0, 0.0], "k": 2}),
    );
    let matches = result.as_array().expect("Expected array");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].get("key").and_then(|v| v.as_str()), Some("v1"));

    // List collections
    let result = call_tool(&mut session, &registry, "strata_vector_list_collections", json!({}));
    let collections = result.as_array().expect("Expected array");
    assert!(collections.iter().any(|c| c.get("name").and_then(|v| v.as_str()) == Some("embeddings")));
}

// =============================================================================
// Transaction Tools
// =============================================================================

#[test]
fn test_transaction_commit() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Begin transaction
    let result = call_tool(&mut session, &registry, "strata_txn_begin", json!({}));
    assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("begun"));

    // Check active
    let result = call_tool(&mut session, &registry, "strata_txn_active", json!({}));
    assert_eq!(result, json!(true));

    // Put data
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "txn-key", "value": "txn-value"}));

    // Commit
    let result = call_tool(&mut session, &registry, "strata_txn_commit", json!({}));
    assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("committed"));

    // Verify data persisted - may be versioned
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "txn-key"}));
    assert_eq!(extract_value(&result), &json!("txn-value"));
}

#[test]
fn test_transaction_rollback() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Put initial data
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "rollback-key", "value": "initial"}));

    // Begin transaction
    call_tool(&mut session, &registry, "strata_txn_begin", json!({}));

    // Modify data
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "rollback-key", "value": "modified"}));

    // Rollback
    let result = call_tool(&mut session, &registry, "strata_txn_rollback", json!({}));
    assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("aborted"));

    // Verify data unchanged - may be versioned
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "rollback-key"}));
    assert_eq!(extract_value(&result), &json!("initial"));
}

// =============================================================================
// Tool Registry
// =============================================================================

#[test]
fn test_tool_count() {
    let registry = ToolRegistry::new();
    let tools = registry.tools();

    // Count should be >= 47 (the planned minimum)
    // We actually have 52 due to additional tools
    assert!(tools.len() >= 47, "Expected at least 47 tools, got {}", tools.len());
}

#[test]
fn test_all_tools_have_required_fields() {
    let registry = ToolRegistry::new();

    for tool in registry.tools() {
        assert!(!tool.name.is_empty(), "Tool name should not be empty");
        assert!(!tool.description.is_empty(), "Tool description should not be empty");
        assert!(tool.name.starts_with("strata_"), "Tool name should start with 'strata_'");
        assert!(tool.input_schema.is_object(), "Tool input_schema should be an object");
    }
}
