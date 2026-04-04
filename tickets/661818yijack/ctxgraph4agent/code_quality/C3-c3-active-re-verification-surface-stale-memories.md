---
id: C3
title: "C3: Active re-verification (surface stale memories for confirmation)"
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: open
owner: apex-agent
phase: C
priority: P2
effort: M
depends_on:
- A1
- A3
created_at: '2026-04-04T08:55:00.000000Z'
updated_at: '2026-04-04T08:55:00.000000Z'
tags:
- c3
- phase-c
- stale
- re-verification
---

<!-- DESCRIPTION -->
Phase C Story 3 (P2, Medium effort, depends on A1+A3). Active re-verification via `get_stale_memories` that surfaces memories approaching TTL expiration. Stale threshold configurable per-agent via `stale_threshold` in MemoryPolicyConfig (default 0.3). Results paginated. Suggested actions based on memory_type. Opt-in (not automatic).

### Acceptance Criteria:
1. `Storage::get_stale_memories(threshold: f64, limit: usize, offset: usize) -> Result<Vec<StaleMemory>>` returns memories with decay_score < threshold, paginated
2. `StaleMemory` struct includes memory content, entity/edge info, decay_score, age, suggested action (renew/update/expire)
3. Suggested action: Facts -> "verify or update", Preferences -> "confirm with user", Experiences -> "let expire", Patterns -> never stale
4. `stale_threshold` defaults to 0.3 but is configurable per-agent in `MemoryPolicyConfig`
5. MCP tool `reverify` returns stale memories
6. CLI: `ctxgraph reverify list` shows stale memories in table
7. CLI: `ctxgraph reverify renew <id>` explicitly renews a memory, bypassing max_renewals

### Technical Requirements:
- Files to modify: types.rs (StaleMemory, StaleAction enum), storage/sqlite.rs (get_stale_memories), ctxgraph-mcp/tools.rs (reverify tool), ctxgraph-cli/commands/reverify.rs
- Add index on `(memory_type, created_at)` for stale memory queries
- Config: `stale_threshold: f64` (default 0.3) in `[policies.<agent>]`
