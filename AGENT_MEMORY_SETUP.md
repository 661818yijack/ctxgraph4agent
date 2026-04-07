# Agent Memory Setup Guide

> Quick setup for integrating ctxgraph4agent as your memory system via MCP

## Prerequisites

1. **ctxgraph4agent compiled** (ask your user to build it if not already done)
2. **Access to the binary** at `~/.hermes/profiles/<profile>/workspace/ctxgraph4agent/target/debug/ctxgraph-mcp`

## Step 1: Create Memory Directory

```bash
mkdir -p ~/.hermes/profiles/<your-profile>/memory_graph
cd ~/.hermes/profiles/<your-profile>/memory_graph
```

## Step 2: Add MCP Server to Config

Edit your `~/.hermes/profiles/<profile>/config.yaml` and add:

```yaml
mcp_servers:
  ctxgraph4agent:
    command: ~/.hermes/profiles/<your-profile>/workspace/ctxgraph4agent/target/debug/ctxgraph-mcp
    args: []
    cwd: ~/.hermes/profiles/<your-profile>/memory_graph
    env:
      RUST_LOG: info
    disabled: false
```

## Step 3: Restart Gateway

Ask your user to restart the Hermes gateway to load the MCP tools.

## Step 4: Test Your Memory

Once restarted, test with:

```python
# Check memory stats
mcp_ctxgraph4agent_stats({})

# Store your first memory
mcp_ctxgraph4agent_add_episode({
    "text": "I am an AI agent using ctxgraph4agent for memory. This is my first stored memory.",
    "source": "setup",
    "tags": ["milestone", "setup"]
})

# Search memories
mcp_ctxgraph4agent_search({
    "query": "memory setup",
    "topk": 5
})
```

## Available Memory Tools

| Tool | Purpose |
|------|---------|
| `mcp_ctxgraph4agent_add_episode` | Store a memory (decisions, facts, experiences) |
| `mcp_ctxgraph4agent_search` | Find memories with BM25 + semantic search |
| `mcp_ctxgraph4agent_traverse` | Navigate the knowledge graph |
| `mcp_ctxgraph4agent_find_precedents` | Find similar past situations |
| `mcp_ctxgraph4agent_learn` | Extract patterns and create skills |
| `mcp_ctxgraph4agent_forget` | Remove outdated memories |
| `mcp_ctxgraph4agent_stats` | Check memory health metrics |
| `mcp_ctxgraph4agent_get_decision` | Retrieve a specific memory by ID |
| `mcp_ctxgraph4agent_list_entities` | List entities in the graph |
| `mcp_ctxgraph4agent_export_graph` | Export all data |

## Best Practices

1. **Tag everything**: Use descriptive tags for better retrieval
2. **Source matters**: Always set `source` (discord, cli, etc.)
3. **Regular learning**: Run `learn` periodically to extract patterns
4. **Check stats**: Monitor memory growth with `stats`

## Troubleshooting

**MCP tools not appearing?**
- Verify the binary path in config.yaml
- Check that the gateway was restarted
- Look for errors in gateway logs

**Permission denied?**
- Make sure the binary is executable: `chmod +x ctxgraph-mcp`

**Database locked?**
- Only one MCP connection per database at a time
- Each agent should have their own `memory_graph` directory

## Multi-Agent Setup

Each agent should have **isolated memory**:

```
~/.hermes/profiles/agent1/memory_graph/  ← Agent 1's memories
~/.hermes/profiles/agent2/memory_graph/  ← Agent 2's memories
```

**Never share** the same `graph.db` between multiple agents - it will cause lock conflicts.

---

*Setup complete! You now have persistent memory across sessions.*
