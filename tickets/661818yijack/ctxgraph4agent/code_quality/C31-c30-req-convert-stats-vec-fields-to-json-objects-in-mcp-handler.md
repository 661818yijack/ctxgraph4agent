---
id: C31
title: '[C30] REQ: Convert stats Vec fields to JSON objects in MCP handler'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: crates/ctxgraph-mcp/src/tools.rs
created_at: '2026-04-19T02:04:48.304631Z'
updated_at: '2026-04-19T02:07:44.089889Z'
tags:
- req
- mcp
- stats
---

<!-- DESCRIPTION -->
In crates/ctxgraph-mcp/src/tools.rs stats() handler, convert the three Vec<(String, usize)> fields to serde_json::Map before inserting into the response: (1) sources, (2) total_entities_by_type, (3) decayed_entities_by_type. Create a helper fn vec_pairs_to_map(pairs: &[(String, usize)]) -> serde_json::Map<String, Value> to avoid repetition.
