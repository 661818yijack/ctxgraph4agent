# CLI Decision Memory Use Case

No API key required: this workflow uses the local SQLite-backed CLI path, so an agent can record and recall decisions before any LLM provider is configured.

## Scenario

An AI agent or team wants a lightweight project memory for architecture decisions. The workflow should answer: "What did we decide, where did it come from, and can I find it again from the terminal?"

## Tested commands

Run these from a scratch project directory:

```bash
ctxgraph init --name decision-memory-demo
ctxgraph log "We chose SQLite for agent memory because it keeps the context graph local, cheap, and easy to back up." \
  --source architecture-notes \
  --tags decision,memory,sqlite
ctxgraph query "SQLite" --source architecture-notes
ctxgraph stats
```

Expected behavior:

1. `ctxgraph init` creates a local `.ctxgraph/graph.db` database.
2. `ctxgraph log` stores the decision as an episode with source and tags.
3. `ctxgraph query` returns the logged SQLite decision from `architecture-notes`.
4. `ctxgraph stats` shows that at least one episode is present.

## When to use this pattern

Use it for agent-readable project memory that must work offline: architecture decisions, incident notes, experiment outcomes, release rationale, and "why did we choose X?" breadcrumbs.

## Notes for agents

- Prefer source names that map to a real artifact, such as `architecture-notes`, `incident-review`, or `pr-summary`.
- Add tags for durable retrieval dimensions (`decision`, `memory`, `database`) instead of only free-text content.
- Keep one decision per `ctxgraph log` call when possible; smaller episodes produce cleaner search results.
- Add an LLM provider later only when cross-domain extraction quality matters; this CLI memory path is useful without one.
