# Agent Memory Workflow Example

This guide shows the smallest Rust SDK workflow an AI agent can use as local memory: record episodes, list recent context, search prior decisions, and traverse graph relationships.

## Run it

```bash
cargo run -p ctxgraph --example agent_memory_workflow
```

The example uses `Graph::in_memory()`, so it does not create files and is safe to run repeatedly.

## What it demonstrates

### Record episodes

Use `Episode::builder(...)` with `source` and `tag` metadata to store agent-observable events as structured memory instead of plain logs.

### List recent memory

Use `graph.list_episodes(limit, offset, after, source)` to load the newest episodes before deciding whether more context is needed.

### Search memory

Use `graph.search(query, limit, source)` to recover prior decisions from full-text search. This is the first retrieval primitive agents should try before asking humans to repeat themselves.

### Traverse graph context

Use `Entity`, `Edge`, and `graph.traverse(start_entity_id, max_depth)` when an agent needs related context beyond the matching episode text.

## Expected shape of output

The exact UUIDs and scores vary, but a successful run prints sections like:

```text
recent episodes: 2
search matches for SQLite: 1
graph context from ctxgraph: 2 entities, 1 edges
```
