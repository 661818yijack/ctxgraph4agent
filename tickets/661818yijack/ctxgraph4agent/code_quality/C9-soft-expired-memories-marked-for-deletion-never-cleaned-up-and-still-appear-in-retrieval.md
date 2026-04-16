---
id: C9
title: Soft-expired memories (marked_for_deletion) never cleaned up and still appear
  in retrieval
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-16T02:03:34.291089Z'
updated_at: '2026-04-16T02:03:34.291089Z'
tags:
- bug
- forget
- soft-expire
- cleanup
- retrieval
---

<!-- DESCRIPTION -->
When a user calls the forget tool with hard=false (soft expire), mark_for_deletion sets metadata.marked_for_deletion=true. However:

1. The cleanup_expired() sweep only checks created_at < cutoff timestamps — it never checks json_extract(metadata, '$.marked_for_deletion'). Soft-expired memories are NEVER deleted by cleanup.

2. The FTS5 search queries in fts_search_entities/fts_search_edges do not filter out marked_for_deletion items. Soft-expired memories STILL APPEAR in search results and retrieval_for_context.

3. retrieve_candidates() does not filter either.

This means forget(hard=false) is essentially a no-op: the data stays visible in search and is never cleaned up. The soft-expire mechanism is completely broken.
