---
id: C14
title: '[C13] REQ: Enrich CLI stats display with all GraphStats fields'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-04-13T02:04:53.862006Z'
updated_at: '2026-04-13T02:09:36.111940Z'
tags:
- req
- cli
- stats
version: 3
---

<!-- DESCRIPTION -->
The CLI stats command (crates/ctxgraph-cli/src/commands/stats.rs) must display ALL fields from GraphStats that the MCP stats tool already surfaces. Currently only 5 of 12 fields are shown. Add: decayed_entities, decayed_edges, last_cleanup_at, queries_since_cleanup, cleanup_interval, cleanup_in_progress, total_entities_by_type, decayed_entities_by_type. Format as a clear health report with sections.
