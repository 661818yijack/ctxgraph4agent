# Phase D — Remaining Gaps

## D3: Cross-session skill sharing

**Gap:** Skills do NOT flow through `enforce_budget` during retrieval.

**Root cause:** `RetrievalCandidate` / `RankedMemory` does not have a `skill_id` field, so skills retrieved via FTS5 cannot enter the budget enforcement pipeline.

**Spec (stories-final.md line 944):**
> "Skills retrieved via FTS5 enter candidate set with floor score 0.8, go through enforce_budget"

**Fix needed:**
1. Add `skill_id: Option<String>` field to `RetrievalCandidate` (types.rs)
2. In `retrieve_for_context` (graph.rs), when skills are retrieved via FTS5, populate `skill_id` on the candidate
3. Ensure skills with floor score 0.8 are passed through `enforce_budget` alongside entities and edges

**Files to modify:**
- `crates/ctxgraph-core/src/types.rs` — RetrievalCandidate / RankedMemory
- `crates/ctxgraph-core/src/graph.rs` — retrieve_for_context (skill candidate creation)

---

## D4: Learn MCP tool

**Gap:** `learn`, `list_skills`, `share_skill` MCP tools not implemented in the MCP server.

**Spec (stories-final.md lines 956-1030):**
- `learn` tool: runs full learning pipeline, returns `{patterns_found, patterns_new, skills_created, skills_updated, skill_ids}`
- `list_skills` tool: returns active (non-superseded) skills for agent
- `share_skill {id}` tool: changes scope to shared

**What subagent DID:**
- `GraphStats` updated with `skill_count`, `pattern_count`, `shared_skill_count`, `private_skill_count`
- `stats()` SQL queries added
- `stats.rs` CLI updated with pattern + skill output
- `Cargo.toml` switched to `rustls-tls`
- `MockPatternDescriber` exported from lib.rs
- `learn.rs` CLI flags verified working

**What subagent DIDN'T finish:**
- `ToolContext` async methods: `tool_learn`, `tool_list_skills`, `tool_share_skill`
- `tools_list()` entries for `learn`, `list_skills`, `share_skill`
- `server.rs` dispatch arms for the 3 new tools

**Fix needed:**

### 1. Add async methods to ToolContext (tools.rs)

```rust
pub async fn tool_learn(&self, args: Value) -> Result<Value, String> {
    // Parse: dry_run, scope, limit, agent from args
    // Call graph.run_learning_pipeline() via graph lock
    // Return LearningOutcome as JSON
}

pub async fn tool_list_skills(&self, args: Value) -> Result<Value, String> {
    // Parse: agent from args
    // Call graph.list_skills(Some(agent)) via graph lock
    // Return skills array as JSON
}

pub async fn tool_share_skill(&self, args: Value) -> Result<Value, String> {
    // Parse: id from args
    // Call graph.storage.share_skill(&id) via graph lock
    // Return {success: true} as JSON
}
```

### 2. Register in tools_list() (tools.rs line ~426)

Add 3 entries to the JSON array:
```json
{
    "name": "learn",
    "description": "Run the full learning pipeline: extract patterns from compression groups, generate descriptions, create skills, supersede old skills.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "dry_run": {"type": "boolean", "description": "Show plan without persisting"},
            "scope": {"type": "string", "description": "'shared' or 'private'"},
            "limit": {"type": "integer", "description": "Max skills to create"},
            "agent": {"type": "string", "description": "Agent ID for ownership"}
        }
    }
},
{
    "name": "list_skills",
    "description": "List active (non-superseded) skills for an agent.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "agent": {"type": "string", "description": "Filter by agent ID (optional)"}
        }
    }
},
{
    "name": "share_skill",
    "description": "Change a skill's scope from private to shared.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "id": {"type": "string", "description": "Skill UUID to share"}
        },
        "required": ["id"]
    }
}
```

### 3. Register in server.rs dispatch

Find the `match tool_name` dispatch and add:
```rust
"learn" => ctx.tool_learn(args).await,
"list_skills" => ctx.tool_list_skills(args).await,
"share_skill" => ctx.tool_share_skill(args).await,
```

**Files to modify:**
- `crates/ctxgraph-mcp/src/tools.rs` — 3 new async methods + tools_list entries
- `crates/ctxgraph-mcp/src/server.rs` — dispatch match arms
- (Optional) `crates/ctxgraph-mcp/src/lib.rs` or `main.rs` — if tools need registration elsewhere

---

## Summary

| Story | Gap | Effort |
|-------|-----|--------|
| D3 | Skills bypass enforce_budget | M |
| D4 | MCP tools learn/list_skills/share_skill missing | M |

Both are medium-effort fixes. D3 is a data flow change (add skill_id to candidate). D4 is straightforward implementation (3 async methods + registration).
