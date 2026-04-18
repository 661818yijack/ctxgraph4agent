---
id: C23
title: '[C22] REQ: Filter marked_for_deletion from list_entities and get_edges_for_entity'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-17T02:04:57.121934Z'
updated_at: '2026-04-18T02:03:16.644127Z'
tags:
- req
- c22
- list_entities
- get_edges_for_entity
---

<!-- DESCRIPTION -->
list_entities() and get_edges_for_entity() do not exclude marked_for_deletion memories. After a user calls forget(hard=false), these functions still return the soft-expired entries. Add WHERE clause filtering COALESCE(json_extract(metadata, '$.marked_for_deletion'), 0) = 0 to both functions in sqlite.rs.
