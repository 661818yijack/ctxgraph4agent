---
id: C26
title: '[C25] REQ: Add soft-expire filter to public search_entities and search_edges'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-18T02:03:32.397286Z'
updated_at: '2026-04-19T02:04:37.395434Z'
tags:
- req
- search
- mcp
---

<!-- DESCRIPTION -->
Fixed in PR #8 (commit 51f1440, merged 2026-04-18). Soft-expire filtering now covers search_entities, search_edges, fts_search, get_1hop_candidates, and cleanup_expired. All paths filter on COALESCE(json_extract(metadata, '$.marked_for_deletion'), 0) = 0.
