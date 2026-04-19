---
id: T24
title: '[C29] UC: Cleanup sweep handles FK safety when deleting soft-expired entities'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:53.429997Z'
updated_at: '2026-04-19T02:04:39.054872Z'
tags:
- uc
- test
- cleanup
- fk-safety
---

<!-- DESCRIPTION -->
Fixed in PR #8 (commit 51f1440, merged 2026-04-18). Soft-expire filtering now covers search_entities, search_edges, fts_search, get_1hop_candidates, and cleanup_expired. All paths filter on COALESCE(json_extract(metadata, '$.marked_for_deletion'), 0) = 0.
