---
id: C4
title: "C4: Re-verify CLI command and MCP tool"
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: open
owner: apex-agent
phase: C
priority: P2
effort: S
depends_on:
- C1
- C2
- C3
created_at: '2026-04-04T08:55:00.000000Z'
updated_at: '2026-04-04T08:55:00.000000Z'
tags:
- c4
- phase-c
- cli
- mcp
- reverify
---

<!-- DESCRIPTION -->
Phase C Story 4 (P2, Small effort, depends on C1+C2+C3). Finalize re-verification CLI and MCP interface wiring up all C1-C3 functionality. CLI gets `ctxgraph reverify` with subcommands `list`, `renew <id>`, `update <id>`, `expire <id>`. MCP gets unified `reverify` tool and a `forget` tool. `ctxgraph stats` includes re-verification stats.

### Acceptance Criteria:
1. CLI: `ctxgraph reverify list --threshold 0.3 --limit 20` lists stale memories with decay_score
2. CLI: `ctxgraph reverify renew <id>` renews a specific memory (resets created_at)
3. CLI: `ctxgraph reverify update <id> --content "new value"` updates memory content in-place
4. CLI: `ctxgraph reverify expire <id>` immediately invalidates a memory (sets valid_until to now)
5. MCP: `reverify` tool with `action: "list" | "renew" | "update" | "expire"` and `id` for targeted actions
6. MCP: `reverify` with `action: "update"` accepts `{id: string, content?: string, memory_type?: string}` — at least one field required
7. MCP: `forget` tool expires a memory by ID with `{"id": "..."}` input
8. `ctxgraph stats` output includes re-verification stats: total stale, total renewed, total expired, total contradicted
9. CLI: `ctxgraph reverify --format json` outputs machine-readable JSON

### Technical Requirements:
- Files to modify: ctxgraph-cli/src/commands/reverify.rs (add renew, update, expire subcommands), ctxgraph-cli/src/commands/mod.rs, ctxgraph-cli/src/main.rs, ctxgraph-mcp/src/tools.rs (reverify, forget tools)
- New: Storage::update_memory, Storage::expire_memory
