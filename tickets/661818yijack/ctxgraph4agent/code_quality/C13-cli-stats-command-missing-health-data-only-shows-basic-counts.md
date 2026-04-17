---
id: C13
title: "CLI stats command missing health data \u2014 only shows basic counts"
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-04-13T02:04:41.420778Z'
updated_at: '2026-04-13T02:09:34.848604Z'
tags:
- stats
- cli
- enhancement
- clippy
- dead-code
version: 2
---

<!-- DESCRIPTION -->
The CLI `ctxgraph stats` command only displays episode/entity/edge counts, sources, and DB size. It ignores all cleanup visibility fields (decayed_entities, decayed_edges, last_cleanup_at, cleanup_interval, queries_since_cleanup, cleanup_in_progress) and type breakdowns (total_entities_by_type, decayed_entities_by_type) that are available in the GraphStats struct and already surfaced by the MCP stats tool. Also, GraphStats is missing contradiction counts that would be useful for monitoring re-verification workload. Additionally, decay_sigmoid() in types.rs is dead code (no callers since Decision was changed to exponential in PR #2), causing clippy -D warnings to fail.
