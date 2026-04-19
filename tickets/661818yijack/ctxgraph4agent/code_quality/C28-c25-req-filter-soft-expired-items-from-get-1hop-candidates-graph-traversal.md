---
id: C28
title: '[C25] REQ: Filter soft-expired items from get_1hop_candidates graph traversal'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-18T02:03:39.936154Z'
updated_at: '2026-04-19T02:04:37.727167Z'
tags:
- req
- retrieval
- graph-traversal
---

<!-- DESCRIPTION -->
Fixed in PR #8 (commit 51f1440, merged 2026-04-18). Soft-expire filtering now covers search_entities, search_edges, fts_search, get_1hop_candidates, and cleanup_expired. All paths filter on COALESCE(json_extract(metadata, '$.marked_for_deletion'), 0) = 0.
