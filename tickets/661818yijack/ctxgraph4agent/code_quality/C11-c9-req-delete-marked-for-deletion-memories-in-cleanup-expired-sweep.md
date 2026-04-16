---
id: C11
title: '[C9] REQ: Delete marked_for_deletion memories in cleanup_expired sweep'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-16T02:03:41.838439Z'
updated_at: '2026-04-16T02:03:41.838439Z'
tags:
- req
- cleanup
- sweep
- soft-expire
---

<!-- DESCRIPTION -->
Add a step in cleanup_expired() that:
1. Queries all entities/edges where json_extract(metadata, '$.marked_for_deletion') = true
2. Deletes them (same FK-safe pattern: edges first, then episode_entities junction, then entities)
3. Counts them in CleanupResult

Soft-expired memories should be cleaned up regardless of their created_at age.
