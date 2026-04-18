---
id: C25
title: Soft-expired memories still leak through search, retrieval pipeline, and cleanup
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-18T02:03:29.160046Z'
updated_at: '2026-04-18T02:03:29.160046Z'
tags:
- bug
- soft-expire
- forget
- search
- retrieval
- cleanup
---

<!-- DESCRIPTION -->
PR #7 (C22/C23/C24) fixed soft-expire filtering for list_entities, list_edges, stats, and edge queries. However, several critical paths still leak marked_for_deletion memories:

1. search_entities() and search_edges() (public MCP API) — no soft-expire filter
2. fts_search_entities(), fts_search_edges(), fts_search_episodes() (retrieval pipeline internals) — no filter
3. get_1hop_candidates() (graph traversal in retrieve_for_context) — no filter  
4. cleanup_expired() — never deletes marked_for_deletion items regardless of age

This means forget(hard=false) is partially broken: soft-expired memories still appear in MCP search results and the retrieval pipeline, and are never cleaned up by the periodic sweep.
