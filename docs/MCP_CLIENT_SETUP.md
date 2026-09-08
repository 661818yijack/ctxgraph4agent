# MCP Client Setup

This guide shows how to connect ctxgraph to an MCP client as a local stdio server.
It is intentionally local-first: **No API key is required** to run the MCP server, add episodes, search, traverse, list entities, export graph data, inspect stats, run local learning over stored episodes, or forget memories. Optional LLM provider keys are only for extraction/learning fallback flows that explicitly call a configured provider.

## 1. Build or install the server

From a source checkout, verify the MCP server binary builds:

```bash
cargo build -p ctxgraph-mcp
```

If you installed ctxgraph from a package manager, verify the binary is on your `PATH`:

```bash
ctxgraph-mcp --help
```

## 2. Configure your MCP client

Add the server as a stdio MCP server. Most clients accept a JSON block shaped like this:

```json
{
  "mcpServers": {
    "ctxgraph": {
      "command": "ctxgraph-mcp"
    }
  }
}
```

To point the server at a specific graph database, either start it with `ctxgraph mcp start --db /path/to/graph.db` when using the CLI wrapper, or set `CTXGRAPH_DB` in the environment for clients that support env overrides.

## 3. Start with a local memory loop

The MCP tool names below are the names exposed by `ctxgraph-mcp`.

### Record context with `add_episode`

Use `add_episode` for decisions, observations, incidents, review notes, or project events:

```json
{
  "text": "We chose SQLite for ctxgraph storage because it keeps agent memory local and portable.",
  "source": "architecture-review",
  "tags": ["storage", "privacy", "decision"]
}
```

### Retrieve context with `search`

Use `search` when the agent needs relevant past episodes:

```json
{
  "query": "Why did we choose SQLite?",
  "limit": 5,
  "source": "architecture-review"
}
```

### Inspect a specific memory with `get_decision`

Use `get_decision` with an episode ID returned by search or add operations:

```json
{
  "id": "episode-id-from-search"
}
```

### Explore relationships with `traverse`

Use `traverse` when you know an entity name and want nearby graph context:

```json
{
  "entity_name": "SQLite",
  "max_depth": 2
}
```

### Traverse multiple seeds with `traverse_batch`

Use `traverse_batch` to avoid multiple client round trips when several entities matter:

```json
{
  "entity_names": ["SQLite", "ctxgraph", "agent memory"],
  "max_depth": 2
}
```

### Find similar cases with `find_precedents`

Use `find_precedents` before a new architecture or implementation decision:

```json
{
  "context": "We need a local storage layer for agent memory with minimal operational overhead.",
  "limit": 5
}
```

### Browse graph contents with `list_entities`

Use `list_entities` for lightweight graph inspection:

```json
{
  "entity_type": "Technology",
  "limit": 25,
  "offset": 0
}
```

### Export data with `export_graph`

Use `export_graph` for audits, backups, or visualizations:

```json
{
  "include_episodes": true,
  "limit": 1000
}
```

### Monitor memory health with `stats`

Use `stats` to inspect episode/entity/edge counts and cleanup state:

```json
{}
```

### Promote repeated patterns with `learn`

Use `learn` after enough episodes accumulate to extract reusable skills:

```json
{
  "agent": "assistant",
  "scope": "private",
  "limit": 20
}
```

### Remove stale memory with `forget`

Use `forget` for explicit lifecycle control. Soft expiration is the default; set `hard` only when immediate deletion is intended:

```json
{
  "id": "memory-id",
  "hard": false
}
```

## 4. Practical agent prompt

After the client is connected, tell the agent when to use ctxgraph:

> Before making architecture or implementation decisions, search ctxgraph for relevant prior context. After important decisions, incidents, or project state changes, add an episode with a short source label and tags.

This keeps memory automatic enough to be useful while avoiding manual bookkeeping for every trivial message.
