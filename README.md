# strata-mcp

MCP (Model Context Protocol) server for [Strata](https://github.com/strata-systems/strata-core) database.

Exposes 47 tools for AI agents to interact with Strata's six data primitives:
KV Store, Event Log, State Cell, JSON Store, Vector Store, and Branches.

## Installation

### From Source

```bash
git clone https://github.com/strata-systems/strata-mcp.git
cd strata-mcp
cargo build --release
```

The binary will be at `target/release/strata-mcp`.

### From crates.io (coming soon)

```bash
cargo install strata-mcp
```

## Usage

### With Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "strata": {
      "command": "/path/to/strata-mcp",
      "args": ["--db", "/path/to/your/data"]
    }
  }
}
```

For ephemeral/testing use (in-memory, no persistence):

```json
{
  "mcpServers": {
    "strata": {
      "command": "/path/to/strata-mcp",
      "args": ["--cache"]
    }
  }
}
```

### Command Line Options

```
strata-mcp [OPTIONS]

Options:
  --db <PATH>     Path to the database directory
  --cache         Use an in-memory database (no persistence)
  --read-only     Open database in read-only mode
  -v, --verbose   Enable debug logging to stderr
  -h, --help      Print help
  -V, --version   Print version
```

## Tools (47 total)

### Key-Value Store (5 tools)

| Tool | Description |
|------|-------------|
| `strata_kv_put` | Store a key-value pair |
| `strata_kv_get` | Get a value by key |
| `strata_kv_delete` | Delete a key |
| `strata_kv_list` | List keys with optional prefix filter |
| `strata_kv_history` | Get version history for a key |

### JSON Document Store (5 tools)

| Tool | Description |
|------|-------------|
| `strata_json_set` | Set a value at a JSONPath |
| `strata_json_get` | Get a value at a JSONPath |
| `strata_json_delete` | Delete a JSON document |
| `strata_json_list` | List JSON document keys |
| `strata_json_history` | Get version history |

### Event Log (4 tools)

| Tool | Description |
|------|-------------|
| `strata_event_append` | Append an event to the log |
| `strata_event_get` | Get an event by sequence number |
| `strata_event_list` | List events by type |
| `strata_event_len` | Get total event count |

### State Cell (7 tools)

| Tool | Description |
|------|-------------|
| `strata_state_set` | Set a state cell value |
| `strata_state_get` | Get a state cell value |
| `strata_state_delete` | Delete a state cell |
| `strata_state_init` | Initialize if not exists |
| `strata_state_cas` | Compare-and-swap update |
| `strata_state_list` | List state cell names |
| `strata_state_history` | Get version history |

### Vector Store (9 tools)

| Tool | Description |
|------|-------------|
| `strata_vector_upsert` | Insert/update a vector |
| `strata_vector_get` | Get a vector by key |
| `strata_vector_delete` | Delete a vector |
| `strata_vector_search` | Similarity search |
| `strata_vector_create_collection` | Create a collection |
| `strata_vector_delete_collection` | Delete a collection |
| `strata_vector_list_collections` | List all collections |
| `strata_vector_stats` | Get collection statistics |
| `strata_vector_batch_upsert` | Batch insert vectors |

### Branch Management (9 tools)

| Tool | Description |
|------|-------------|
| `strata_branch_create` | Create a new branch |
| `strata_branch_get` | Get branch info |
| `strata_branch_list` | List all branches |
| `strata_branch_exists` | Check if branch exists |
| `strata_branch_delete` | Delete a branch |
| `strata_branch_fork` | Fork current branch |
| `strata_branch_diff` | Diff two branches |
| `strata_branch_merge` | Merge branches |
| `strata_branch_switch` | Switch current branch |

### Space Management (4 tools)

| Tool | Description |
|------|-------------|
| `strata_space_list` | List spaces in branch |
| `strata_space_create` | Create a space |
| `strata_space_delete` | Delete a space |
| `strata_space_switch` | Switch current space |

### Transaction Control (5 tools)

| Tool | Description |
|------|-------------|
| `strata_txn_begin` | Begin a transaction |
| `strata_txn_commit` | Commit transaction |
| `strata_txn_rollback` | Rollback transaction |
| `strata_txn_info` | Get transaction info |
| `strata_txn_active` | Check if transaction active |

### Database Operations (4 tools)

| Tool | Description |
|------|-------------|
| `strata_db_ping` | Check connectivity |
| `strata_db_info` | Get database info |
| `strata_db_flush` | Flush writes to disk |
| `strata_db_compact` | Trigger compaction |

## Session State

The MCP server maintains session state that persists across tool calls:

- **Current Branch**: Set with `strata_branch_switch`, defaults to "default"
- **Current Space**: Set with `strata_space_switch`, defaults to "default"
- **Transaction State**: Tracked via `strata_txn_*` tools

All data operations use the current branch/space context automatically.

## Example Conversation

```
User: Store my preferences
Agent: [calls strata_kv_put with key="preferences", value={"theme": "dark"}]

User: Create a branch for experiments
Agent: [calls strata_branch_create with branch_id="experiment"]
Agent: [calls strata_branch_switch with branch="experiment"]

User: Try a different theme setting
Agent: [calls strata_kv_put with key="preferences", value={"theme": "light"}]

User: Actually, let's discard that and go back
Agent: [calls strata_branch_switch with branch="default"]
# Original preferences are intact
```

## Protocol

The server implements [MCP](https://modelcontextprotocol.io/) over JSON-RPC 2.0 on stdin/stdout.

Supported methods:
- `initialize` - Initialize the server
- `tools/list` - List available tools
- `tools/call` - Execute a tool
- `ping` - Health check

## Development

```bash
# Run tests
cargo test

# Run with verbose logging
./target/release/strata-mcp --cache -v

# Test with a JSON-RPC request
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./target/release/strata-mcp --cache
```

## License

MIT
