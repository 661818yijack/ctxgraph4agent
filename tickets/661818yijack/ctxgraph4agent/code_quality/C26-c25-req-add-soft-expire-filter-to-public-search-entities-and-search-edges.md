---
id: C26
title: '[C25] REQ: Add soft-expire filter to public search_entities and search_edges'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-18T02:03:32.397286Z'
updated_at: '2026-04-18T02:03:32.397286Z'
tags:
- req
- search
- mcp
---

<!-- DESCRIPTION -->
The public search_entities() (line 640) and search_edges() functions used by the MCP search tool do not filter out marked_for_deletion memories. Add COALESCE(json_extract(metadata, '$.marked_for_deletion'), 0) = 0 to the WHERE clause of both functions. These are the user-facing search API — if soft-expired items appear here, users see data they explicitly asked to forget.
