# MCP cleanup stats guide

This guide shows how an agent can inspect ctxgraph maintenance health through the MCP `stats` tool without making any LLM calls or downloading model files.

## Why this matters

Long-running agent memory stores need two simple signals:

1. how much graph data exists, and
2. whether cleanup is healthy or due.

The MCP `stats` response exposes both. Use it before deciding to run expensive retrieval, cleanup, or reverify workflows.

## Local setup

The stats path only reads SQLite metadata, so it works offline.

```bash
ctxgraph init
ctxgraph log "Chose PostgreSQL for billing because it handles concurrent writes."
ctxgraph stats
```

For MCP clients, call the `stats` tool with an empty argument object:

```json
{
  "name": "stats",
  "arguments": {}
}
```

## Cleanup fields

The `stats` tool includes these maintenance fields:

- `last_cleanup_at`: RFC3339 timestamp for the last cleanup sweep, or `"never"` on a fresh graph.
- `queries_since_cleanup`: number of queries counted since the last cleanup sweep.
- `cleanup_interval`: configured query interval between lazy cleanup sweeps.
- `next_cleanup_in`: non-negative number of queries remaining before lazy cleanup is due.
- `cleanup_in_progress`: true only while a cleanup sweep is already running.

On a fresh graph, agents should expect:

```json
{
  "last_cleanup_at": "never",
  "queries_since_cleanup": 0,
  "cleanup_interval": 100,
  "next_cleanup_in": 100,
  "cleanup_in_progress": false
}
```

## Agent decision pattern

Use this lightweight check before maintenance decisions:

1. Call MCP `stats`.
2. If `cleanup_in_progress` is true, do not start another cleanup.
3. If `next_cleanup_in` is `0`, cleanup is due or overdue.
4. If `last_cleanup_at` is `"never"` on a large graph, schedule or manually run cleanup during a quiet period.
5. Treat `sources`, `total_entities_by_type`, and `decayed_entities_by_type` as JSON objects keyed by source/type names, not arrays.

## Verification

This guide is backed by the focused MCP test:

```bash
cargo test -p ctxgraph-mcp --test stats_serialization_test test_stats_cleanup_fields -- --nocapture
```

That test asserts the fresh-graph defaults and client-friendly JSON types for cleanup health fields.
