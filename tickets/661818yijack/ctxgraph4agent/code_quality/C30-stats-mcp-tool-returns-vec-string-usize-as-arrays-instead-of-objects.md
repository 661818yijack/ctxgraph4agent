---
id: C30
title: Stats MCP tool returns Vec<(String,usize)> as arrays instead of objects
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: crates/ctxgraph-mcp/src/tools.rs
created_at: '2026-04-19T02:04:42.532333Z'
updated_at: '2026-04-19T02:07:43.924104Z'
tags:
- bug
- mcp
- stats
- serialization
---

<!-- DESCRIPTION -->
The stats MCP tool serializes Vec<(String, usize)> fields (sources, total_entities_by_type, decayed_entities_by_type) as arrays of arrays [["fact", 5], ...] instead of objects {"fact": 5, ...}. This makes the response harder for agents to consume programmatically. The fix is to convert these to serde_json::Map in the MCP tools.rs stats() handler. Affected fields: sources, total_entities_by_type, decayed_entities_by_type.
