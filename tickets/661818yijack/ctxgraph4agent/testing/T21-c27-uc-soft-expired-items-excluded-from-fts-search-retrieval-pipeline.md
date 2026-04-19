---
id: T21
title: '[C27] UC: Soft-expired items excluded from fts_search retrieval pipeline'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:52.948406Z'
updated_at: '2026-04-19T02:04:38.561151Z'
tags:
- uc
- test
- retrieval
- fts5
---

<!-- DESCRIPTION -->
Fixed in PR #8 (commit 51f1440, merged 2026-04-18). Soft-expire filtering now covers search_entities, search_edges, fts_search, get_1hop_candidates, and cleanup_expired. All paths filter on COALESCE(json_extract(metadata, '$.marked_for_deletion'), 0) = 0.
