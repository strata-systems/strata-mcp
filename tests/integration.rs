//! Integration tests for the MCP server.

use serde_json::{json, Map, Value as JsonValue};
use strata_mcp::{McpSession, ToolRegistry};
use stratadb::Strata;

/// Create a test session with an in-memory database.
fn test_session() -> McpSession {
    let db = Strata::cache().expect("Failed to create cache database");
    McpSession::new(db)
}

/// Create a read-only test session.
fn read_only_session() -> McpSession {
    use stratadb::{AccessMode, OpenOptions};
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    // First open read-write to create the database
    {
        let db = Strata::open_with(dir.path(), OpenOptions::new()).expect("Failed to create db");
        let mut s = db.session();
        s.execute(stratadb::Command::KvPut {
            branch: None,
            space: None,
            key: "test".to_string(),
            value: stratadb::Value::String("hello".to_string()),
        })
        .expect("Failed to put");
    }
    // Re-open read-only
    let db = Strata::open_with(dir.path(), OpenOptions::new().access_mode(AccessMode::ReadOnly))
        .expect("Failed to open read-only");
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
    registry
        .dispatch(session, name, args_map)
        .unwrap_or_else(|e| panic!("Tool {} failed: {}", name, e))
}

/// Helper to dispatch a tool call and expect an error.
fn call_tool_err(
    session: &mut McpSession,
    registry: &ToolRegistry,
    name: &str,
    args: JsonValue,
) -> strata_mcp::McpError {
    let args_map: Map<String, JsonValue> = match args {
        JsonValue::Object(m) => m,
        _ => Map::new(),
    };
    registry
        .dispatch(session, name, args_map)
        .expect_err(&format!("Expected tool {} to fail", name))
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

#[test]
fn test_db_flush() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(&mut session, &registry, "strata_db_flush", json!({}));
    assert_eq!(result, json!(null));
}

#[test]
fn test_db_compact() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(&mut session, &registry, "strata_db_compact", json!({}));
    assert_eq!(result, json!(null));
}

// =============================================================================
// KV Tools
// =============================================================================

#[test]
fn test_kv_put_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_put",
        json!({"key": "test_key", "value": "hello world"}),
    );
    assert!(result.get("version").is_some());

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

    call_tool(
        &mut session,
        &registry,
        "strata_kv_put",
        json!({"key": "to_delete", "value": 42}),
    );

    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_delete",
        json!({"key": "to_delete"}),
    );
    assert_eq!(result, json!(true));

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

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "user:1", "value": "alice"}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "user:2", "value": "bob"}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "item:1", "value": "book"}));

    let result = call_tool(&mut session, &registry, "strata_kv_list", json!({}));
    let keys = result.as_array().expect("Expected array");
    assert_eq!(keys.len(), 3);

    let result = call_tool(&mut session, &registry, "strata_kv_list", json!({"prefix": "user:"}));
    let keys = result.as_array().expect("Expected array");
    assert_eq!(keys.len(), 2);
}

#[test]
fn test_kv_history() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "evolving", "value": 1}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "evolving", "value": 2}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "evolving", "value": 3}));

    let result = call_tool(&mut session, &registry, "strata_kv_history", json!({"key": "evolving"}));
    let history = result.as_array().expect("Expected array of versions");
    assert!(history.len() >= 2, "Expected at least 2 versions");
}

#[test]
fn test_kv_put_many_get_many() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_put_many",
        json!({"items": [
            {"key": "batch:1", "value": "a"},
            {"key": "batch:2", "value": "b"},
            {"key": "batch:3", "value": "c"}
        ]}),
    );
    let versions = result.as_array().expect("Expected array");
    assert_eq!(versions.len(), 3);

    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_get_many",
        json!({"keys": ["batch:1", "batch:2", "batch:3"]}),
    );
    let values = result.as_array().expect("Expected array");
    assert_eq!(values.len(), 3);
    assert_eq!(extract_value(&values[0]), &json!("a"));
}

#[test]
fn test_kv_delete_many() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "dm:1", "value": 1}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "dm:2", "value": 2}));

    let result = call_tool(
        &mut session,
        &registry,
        "strata_kv_delete_many",
        json!({"keys": ["dm:1", "dm:2"]}),
    );
    let results = result.as_array().expect("Expected array");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0], json!(true));
    assert_eq!(results[1], json!(true));
}

// =============================================================================
// State Tools
// =============================================================================

#[test]
fn test_state_set_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_state_set",
        json!({"cell": "counter", "value": 100}),
    );
    assert!(result.get("version").is_some());

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

    call_tool(
        &mut session,
        &registry,
        "strata_state_init",
        json!({"cell": "status", "value": "pending"}),
    );

    let result = call_tool(
        &mut session,
        &registry,
        "strata_state_get",
        json!({"cell": "status"}),
    );
    assert_eq!(extract_value(&result), &json!("pending"));
}

#[test]
fn test_state_delete() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_state_set", json!({"cell": "temp", "value": 1}));
    let result = call_tool(&mut session, &registry, "strata_state_delete", json!({"cell": "temp"}));
    assert_eq!(result, json!(true));

    let result = call_tool(&mut session, &registry, "strata_state_get", json!({"cell": "temp"}));
    assert_eq!(result, json!(null));
}

#[test]
fn test_state_list() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_state_set", json!({"cell": "cfg:a", "value": 1}));
    call_tool(&mut session, &registry, "strata_state_set", json!({"cell": "cfg:b", "value": 2}));

    let result = call_tool(&mut session, &registry, "strata_state_list", json!({"prefix": "cfg:"}));
    let cells = result.as_array().expect("Expected array");
    assert_eq!(cells.len(), 2);
}

#[test]
fn test_state_cas() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let v = call_tool(&mut session, &registry, "strata_state_set", json!({"cell": "lock", "value": "free"}));
    let version = v.get("version").and_then(|v| v.as_u64()).unwrap();

    // CAS with matching expected_counter should succeed
    let result = call_tool(
        &mut session,
        &registry,
        "strata_state_cas",
        json!({"cell": "lock", "value": "taken", "expected_counter": version}),
    );
    // Result is a version number (success) or null (CAS failure)
    assert!(result.is_number(), "Expected version number, got: {:?}", result);
}

#[test]
fn test_state_history() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_state_set", json!({"cell": "ver", "value": 1}));
    call_tool(&mut session, &registry, "strata_state_set", json!({"cell": "ver", "value": 2}));

    let result = call_tool(&mut session, &registry, "strata_state_history", json!({"cell": "ver"}));
    let history = result.as_array().expect("Expected array");
    assert!(history.len() >= 2);
}

// =============================================================================
// Event Tools
// =============================================================================

#[test]
fn test_event_append_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_event_append",
        json!({"event_type": "user_action", "payload": {"action": "click", "target": "button"}}),
    );
    assert!(result.get("version").is_some());

    let result = call_tool(&mut session, &registry, "strata_event_len", json!({}));
    assert_eq!(result, json!(1));
}

#[test]
fn test_event_get_by_sequence() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(
        &mut session,
        &registry,
        "strata_event_append",
        json!({"event_type": "log", "payload": {"msg": "first"}}),
    );

    let result = call_tool(&mut session, &registry, "strata_event_get", json!({"sequence": 0}));
    assert!(!result.is_null());
}

#[test]
fn test_event_list_by_type() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_event_append", json!({"event_type": "a", "payload": {"n": 1}}));
    call_tool(&mut session, &registry, "strata_event_append", json!({"event_type": "b", "payload": {"n": 2}}));
    call_tool(&mut session, &registry, "strata_event_append", json!({"event_type": "a", "payload": {"n": 3}}));

    let result = call_tool(&mut session, &registry, "strata_event_list", json!({"event_type": "a"}));
    let events = result.as_array().expect("Expected array");
    assert_eq!(events.len(), 2);
}

#[test]
fn test_event_list_paginated() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    for i in 0..5 {
        call_tool(&mut session, &registry, "strata_event_append", json!({"event_type": "pg", "payload": {"i": i}}));
    }

    let result = call_tool(
        &mut session,
        &registry,
        "strata_event_list",
        json!({"event_type": "pg", "limit": 2}),
    );
    let events = result.as_array().expect("Expected array");
    assert_eq!(events.len(), 2);
}

// =============================================================================
// JSON Tools
// =============================================================================

#[test]
fn test_json_set_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(
        &mut session,
        &registry,
        "strata_json_set",
        json!({"key": "config", "path": "$", "value": {"theme": "dark", "lang": "en"}}),
    );

    let result = call_tool(
        &mut session,
        &registry,
        "strata_json_get",
        json!({"key": "config", "path": "$"}),
    );
    let value = extract_value(&result);
    assert_eq!(value.get("theme").and_then(|v| v.as_str()), Some("dark"));

    let result = call_tool(
        &mut session,
        &registry,
        "strata_json_get",
        json!({"key": "config", "path": "$.theme"}),
    );
    assert_eq!(extract_value(&result), &json!("dark"));
}

#[test]
fn test_json_delete() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_json_set", json!({"key": "temp", "path": "$", "value": 42}));
    let result = call_tool(&mut session, &registry, "strata_json_delete", json!({"key": "temp", "path": "$"}));
    assert!(result.is_number());
}

#[test]
fn test_json_list() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_json_set", json!({"key": "doc:a", "path": "$", "value": 1}));
    call_tool(&mut session, &registry, "strata_json_set", json!({"key": "doc:b", "path": "$", "value": 2}));

    let result = call_tool(&mut session, &registry, "strata_json_list", json!({"prefix": "doc:"}));
    let keys = result.get("keys").and_then(|v| v.as_array()).expect("Expected keys array");
    assert_eq!(keys.len(), 2);
}

#[test]
fn test_json_history() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_json_set", json!({"key": "versioned", "path": "$", "value": "v1"}));
    call_tool(&mut session, &registry, "strata_json_set", json!({"key": "versioned", "path": "$", "value": "v2"}));

    let result = call_tool(&mut session, &registry, "strata_json_history", json!({"key": "versioned"}));
    let history = result.as_array().expect("Expected array");
    assert!(history.len() >= 2);
}

// =============================================================================
// Branch Tools
// =============================================================================

#[test]
fn test_branch_create_list() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_branch_create",
        json!({"branch_id": "test-branch"}),
    );
    assert_eq!(result.get("id").and_then(|v| v.as_str()), Some("test-branch"));

    let result = call_tool(&mut session, &registry, "strata_branch_list", json!({}));
    let branches = result.as_array().expect("Expected array");
    assert!(branches.len() >= 2);

    let result = call_tool(
        &mut session,
        &registry,
        "strata_branch_exists",
        json!({"branch": "test-branch"}),
    );
    assert_eq!(result, json!(true));
}

#[test]
fn test_branch_create_with_metadata() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_branch_create",
        json!({"branch_id": "meta-branch", "metadata": {"purpose": "experiment"}}),
    );
    assert_eq!(result.get("id").and_then(|v| v.as_str()), Some("meta-branch"));
}

#[test]
fn test_branch_switch() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_branch_create", json!({"branch_id": "feature"}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "x", "value": 1}));

    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "feature"}));
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "x"}));
    assert_eq!(result, json!(null));

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "x", "value": 2}));
    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "default"}));

    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "x"}));
    assert_eq!(extract_value(&result), &json!(1));
}

#[test]
fn test_branch_fork() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "shared", "value": "original"}));

    let result = call_tool(&mut session, &registry, "strata_branch_fork", json!({"destination": "forked"}));
    assert!(result.get("keys_copied").is_some());

    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "forked"}));
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "shared"}));
    assert_eq!(extract_value(&result), &json!("original"));
}

#[test]
fn test_branch_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(&mut session, &registry, "strata_branch_get", json!({"branch": "default"}));
    assert_eq!(result.get("id").and_then(|v| v.as_str()), Some("default"));
    assert!(result.get("status").is_some());
    assert!(result.get("version").is_some());
}

#[test]
fn test_branch_delete() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_branch_create", json!({"branch_id": "to-delete"}));
    call_tool(&mut session, &registry, "strata_branch_delete", json!({"branch": "to-delete"}));

    let result = call_tool(&mut session, &registry, "strata_branch_exists", json!({"branch": "to-delete"}));
    assert_eq!(result, json!(false));
}

#[test]
fn test_branch_diff() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "a", "value": 1}));
    call_tool(&mut session, &registry, "strata_branch_create", json!({"branch_id": "diff-target"}));

    let result = call_tool(
        &mut session,
        &registry,
        "strata_branch_diff",
        json!({"branch_a": "default", "branch_b": "diff-target"}),
    );
    assert!(result.get("summary").is_some());
}

#[test]
fn test_branch_merge() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_branch_create", json!({"branch_id": "merge-src"}));
    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "merge-src"}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "merged", "value": "from-src"}));
    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "default"}));

    let result = call_tool(
        &mut session,
        &registry,
        "strata_branch_merge",
        json!({"source": "merge-src"}),
    );
    assert!(result.get("keys_applied").is_some());
}

// =============================================================================
// Space Tools
// =============================================================================

#[test]
fn test_space_operations() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_space_create", json!({"space": "my-space"}));

    let result = call_tool(&mut session, &registry, "strata_space_list", json!({}));
    let spaces = result.as_array().expect("Expected array");
    assert!(spaces.iter().any(|s| s.as_str() == Some("my-space")));

    call_tool(&mut session, &registry, "strata_space_switch", json!({"space": "my-space"}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "space-key", "value": "space-value"}));

    call_tool(&mut session, &registry, "strata_space_switch", json!({"space": "default"}));
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "space-key"}));
    assert_eq!(result, json!(null));
}

#[test]
fn test_space_exists() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(&mut session, &registry, "strata_space_exists", json!({"space": "default"}));
    assert_eq!(result, json!(true));

    let result = call_tool(&mut session, &registry, "strata_space_exists", json!({"space": "nonexistent"}));
    assert_eq!(result, json!(false));
}

#[test]
fn test_space_delete() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_space_create", json!({"space": "to-remove"}));
    call_tool(&mut session, &registry, "strata_space_delete", json!({"space": "to-remove", "force": true}));

    let result = call_tool(&mut session, &registry, "strata_space_exists", json!({"space": "to-remove"}));
    assert_eq!(result, json!(false));
}

// =============================================================================
// Vector Tools
// =============================================================================

#[test]
fn test_vector_operations() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(
        &mut session,
        &registry,
        "strata_vector_create_collection",
        json!({"collection": "embeddings", "dimension": 4, "metric": "cosine"}),
    );

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

    let result = call_tool(
        &mut session,
        &registry,
        "strata_vector_search",
        json!({"collection": "embeddings", "query": [1.0, 0.0, 0.0, 0.0], "k": 2}),
    );
    let matches = result.as_array().expect("Expected array");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].get("key").and_then(|v| v.as_str()), Some("v1"));

    let result = call_tool(&mut session, &registry, "strata_vector_list_collections", json!({}));
    let collections = result.as_array().expect("Expected array");
    assert!(collections
        .iter()
        .any(|c| c.get("name").and_then(|v| v.as_str()) == Some("embeddings")));
}

#[test]
fn test_vector_get() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_vector_create_collection", json!({"collection": "vget", "dimension": 3}));
    call_tool(&mut session, &registry, "strata_vector_upsert", json!({"collection": "vget", "key": "k1", "vector": [1.0, 2.0, 3.0], "metadata": {"label": "test"}}));

    let result = call_tool(&mut session, &registry, "strata_vector_get", json!({"collection": "vget", "key": "k1"}));
    assert!(result.get("embedding").is_some());
    assert!(result.get("version").is_some());
}

#[test]
fn test_vector_delete() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_vector_create_collection", json!({"collection": "vdel", "dimension": 2}));
    call_tool(&mut session, &registry, "strata_vector_upsert", json!({"collection": "vdel", "key": "k1", "vector": [1.0, 2.0]}));

    let result = call_tool(&mut session, &registry, "strata_vector_delete", json!({"collection": "vdel", "key": "k1"}));
    assert_eq!(result, json!(true));

    let result = call_tool(&mut session, &registry, "strata_vector_get", json!({"collection": "vdel", "key": "k1"}));
    assert_eq!(result, json!(null));
}

#[test]
fn test_vector_delete_collection() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_vector_create_collection", json!({"collection": "temp_coll", "dimension": 2}));
    let result = call_tool(&mut session, &registry, "strata_vector_delete_collection", json!({"collection": "temp_coll"}));
    assert_eq!(result, json!(true));
}

#[test]
fn test_vector_stats() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_vector_create_collection", json!({"collection": "stats_coll", "dimension": 4}));
    call_tool(&mut session, &registry, "strata_vector_upsert", json!({"collection": "stats_coll", "key": "s1", "vector": [1.0, 0.0, 0.0, 0.0]}));

    let result = call_tool(&mut session, &registry, "strata_vector_stats", json!({"collection": "stats_coll"}));
    // VectorCollectionStats returns VectorCollectionList (array with single entry)
    let stats = if result.is_array() {
        result.as_array().unwrap().first().expect("Expected at least one stats entry").clone()
    } else {
        result
    };
    assert_eq!(stats.get("dimension").and_then(|v| v.as_u64()), Some(4));
    assert_eq!(stats.get("count").and_then(|v| v.as_u64()), Some(1));
}

#[test]
fn test_vector_batch_upsert() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_vector_create_collection", json!({"collection": "batch", "dimension": 2}));

    let result = call_tool(
        &mut session,
        &registry,
        "strata_vector_batch_upsert",
        json!({"collection": "batch", "entries": [
            {"key": "b1", "vector": [1.0, 0.0]},
            {"key": "b2", "vector": [0.0, 1.0], "metadata": {"tag": "second"}}
        ]}),
    );
    let versions = result.as_array().expect("Expected array");
    assert_eq!(versions.len(), 2);
}

#[test]
fn test_vector_search_filtered() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_vector_create_collection", json!({"collection": "filtered", "dimension": 2}));
    call_tool(&mut session, &registry, "strata_vector_upsert", json!({"collection": "filtered", "key": "f1", "vector": [1.0, 0.0], "metadata": {"color": "red"}}));
    call_tool(&mut session, &registry, "strata_vector_upsert", json!({"collection": "filtered", "key": "f2", "vector": [0.9, 0.1], "metadata": {"color": "blue"}}));

    let result = call_tool(
        &mut session,
        &registry,
        "strata_vector_search",
        json!({
            "collection": "filtered",
            "query": [1.0, 0.0],
            "k": 10,
            "filter": [{"field": "color", "op": "eq", "value": "red"}]
        }),
    );
    let matches = result.as_array().expect("Expected array");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].get("key").and_then(|v| v.as_str()), Some("f1"));
}

// =============================================================================
// Transaction Tools
// =============================================================================

#[test]
fn test_transaction_commit() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(&mut session, &registry, "strata_txn_begin", json!({}));
    assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("begun"));

    let result = call_tool(&mut session, &registry, "strata_txn_active", json!({}));
    assert_eq!(result, json!(true));

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "txn-key", "value": "txn-value"}));

    let result = call_tool(&mut session, &registry, "strata_txn_commit", json!({}));
    assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("committed"));

    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "txn-key"}));
    assert_eq!(extract_value(&result), &json!("txn-value"));
}

#[test]
fn test_transaction_rollback() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "rollback-key", "value": "initial"}));

    call_tool(&mut session, &registry, "strata_txn_begin", json!({}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "rollback-key", "value": "modified"}));

    let result = call_tool(&mut session, &registry, "strata_txn_rollback", json!({}));
    assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("aborted"));

    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "rollback-key"}));
    assert_eq!(extract_value(&result), &json!("initial"));
}

#[test]
fn test_transaction_info() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // No active transaction
    let result = call_tool(&mut session, &registry, "strata_txn_info", json!({}));
    assert_eq!(result, json!(null));

    // Begin transaction
    call_tool(&mut session, &registry, "strata_txn_begin", json!({}));
    let result = call_tool(&mut session, &registry, "strata_txn_info", json!({}));
    assert!(result.get("id").is_some());
    assert!(result.get("started_at").is_some());

    call_tool(&mut session, &registry, "strata_txn_rollback", json!({}));
}

#[test]
fn test_transaction_read_only() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_txn_begin", json!({"read_only": true}));
    let result = call_tool(&mut session, &registry, "strata_txn_active", json!({}));
    assert_eq!(result, json!(true));

    call_tool(&mut session, &registry, "strata_txn_rollback", json!({}));
}

// =============================================================================
// Bundle Tools
// =============================================================================

#[test]
fn test_bundle_export_import() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    // Create a separate branch for export to avoid "default already exists" on import
    call_tool(&mut session, &registry, "strata_branch_create", json!({"branch_id": "export-branch"}));
    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "export-branch"}));
    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "export-key", "value": "export-value"}));

    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("test.bundle");
    let path_str = path.to_str().unwrap();

    // Export
    let result = call_tool(
        &mut session,
        &registry,
        "strata_bundle_export",
        json!({"branch": "export-branch", "path": path_str}),
    );
    assert!(result.get("entry_count").is_some());

    // Validate
    let result = call_tool(
        &mut session,
        &registry,
        "strata_bundle_validate",
        json!({"path": path_str}),
    );
    assert_eq!(result.get("checksums_valid").and_then(|v| v.as_bool()), Some(true));

    // Delete the branch so import can re-create it
    call_tool(&mut session, &registry, "strata_branch_switch", json!({"branch": "default"}));
    call_tool(&mut session, &registry, "strata_branch_delete", json!({"branch": "export-branch"}));

    // Import
    let result = call_tool(
        &mut session,
        &registry,
        "strata_bundle_import",
        json!({"path": path_str}),
    );
    assert!(result.get("keys_written").is_some());
}

// =============================================================================
// Retention Tool
// =============================================================================

#[test]
fn test_retention_apply() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(&mut session, &registry, "strata_retention_apply", json!({}));
    assert_eq!(result, json!(null));
}

// =============================================================================
// Search Tool
// =============================================================================

#[test]
fn test_search() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "search-key", "value": "searchable text content"}));

    let result = call_tool(
        &mut session,
        &registry,
        "strata_search",
        json!({"query": "searchable", "k": 5}),
    );
    // Result is an array of search hits
    assert!(result.is_array());
}

#[test]
fn test_search_empty_database() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_search",
        json!({"query": "nothing"}),
    );
    assert_eq!(result, json!([]));
}

#[test]
fn test_search_with_primitives_filter() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    call_tool(&mut session, &registry, "strata_kv_put", json!({"key": "k1", "value": "data"}));

    let result = call_tool(
        &mut session,
        &registry,
        "strata_search",
        json!({"query": "data", "primitives": ["kv"]}),
    );
    assert!(result.is_array());
}

#[test]
fn test_search_with_mode() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_search",
        json!({"query": "test", "mode": "keyword"}),
    );
    assert!(result.is_array());
}

#[test]
fn test_search_with_expand_rerank_disabled() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_search",
        json!({"query": "test", "expand": false, "rerank": false}),
    );
    assert!(result.is_array());
}

#[test]
fn test_search_with_time_range() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_search",
        json!({
            "query": "test",
            "time_range": {"start": "2020-01-01T00:00:00Z", "end": "2030-01-01T00:00:00Z"}
        }),
    );
    assert!(result.is_array());
}

#[test]
fn test_search_with_all_options() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let result = call_tool(
        &mut session,
        &registry,
        "strata_search",
        json!({
            "query": "hello",
            "k": 5,
            "primitives": ["kv"],
            "mode": "hybrid",
            "expand": false,
            "rerank": false
        }),
    );
    assert!(result.is_array());
}

// =============================================================================
// Read-Only Mode
// =============================================================================

#[test]
fn test_read_only_rejects_writes() {
    let mut session = read_only_session();
    let registry = ToolRegistry::new();

    // Read should work
    let result = call_tool(&mut session, &registry, "strata_kv_get", json!({"key": "test"}));
    assert_eq!(extract_value(&result), &json!("hello"));

    // Write should fail
    let err = call_tool_err(
        &mut session,
        &registry,
        "strata_kv_put",
        json!({"key": "new", "value": "fail"}),
    );
    let err_str = format!("{}", err);
    assert!(
        err_str.contains("read-only") || err_str.contains("ACCESS_DENIED"),
        "Expected read-only error, got: {}",
        err_str
    );
}

#[test]
fn test_read_only_allows_reads() {
    let mut session = read_only_session();
    let registry = ToolRegistry::new();

    // All read operations should work
    call_tool(&mut session, &registry, "strata_db_ping", json!({}));
    call_tool(&mut session, &registry, "strata_db_info", json!({}));
    call_tool(&mut session, &registry, "strata_kv_list", json!({}));
    call_tool(&mut session, &registry, "strata_branch_list", json!({}));
    call_tool(&mut session, &registry, "strata_space_list", json!({}));
}

#[test]
fn test_read_only_rejects_state_write() {
    let mut session = read_only_session();
    let registry = ToolRegistry::new();

    let err = call_tool_err(
        &mut session,
        &registry,
        "strata_state_set",
        json!({"cell": "c", "value": 1}),
    );
    let err_str = format!("{}", err);
    assert!(err_str.contains("read-only") || err_str.contains("ACCESS_DENIED"));
}

#[test]
fn test_read_only_rejects_event_append() {
    let mut session = read_only_session();
    let registry = ToolRegistry::new();

    let err = call_tool_err(
        &mut session,
        &registry,
        "strata_event_append",
        json!({"event_type": "test", "payload": 1}),
    );
    let err_str = format!("{}", err);
    assert!(err_str.contains("read-only") || err_str.contains("ACCESS_DENIED"));
}

// =============================================================================
// Error Handling
// =============================================================================

#[test]
fn test_unknown_tool() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let err = call_tool_err(&mut session, &registry, "strata_nonexistent", json!({}));
    let err_str = format!("{}", err);
    assert!(err_str.contains("unknown tool"));
}

#[test]
fn test_missing_required_arg() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let err = call_tool_err(&mut session, &registry, "strata_kv_put", json!({}));
    let err_str = format!("{}", err);
    assert!(err_str.contains("key") || err_str.contains("missing"));
}

#[test]
fn test_branch_not_found() {
    let mut session = test_session();
    let registry = ToolRegistry::new();

    let err = call_tool_err(
        &mut session,
        &registry,
        "strata_branch_switch",
        json!({"branch": "does-not-exist"}),
    );
    let err_str = format!("{}", err);
    assert!(err_str.contains("not found"));
}

// =============================================================================
// Tool Registry
// =============================================================================

#[test]
fn test_tool_count() {
    let registry = ToolRegistry::new();
    let tools = registry.tools();

    // After adding time_range + configure_model: 63 total
    assert_eq!(
        tools.len(),
        63,
        "Expected 63 tools, got {}. Tools: {:?}",
        tools.len(),
        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
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

#[test]
fn test_no_duplicate_tool_names() {
    let registry = ToolRegistry::new();
    let tools = registry.tools();
    let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    let original_count = names.len();
    names.sort();
    names.dedup();
    assert_eq!(
        names.len(),
        original_count,
        "Found duplicate tool names"
    );
}
