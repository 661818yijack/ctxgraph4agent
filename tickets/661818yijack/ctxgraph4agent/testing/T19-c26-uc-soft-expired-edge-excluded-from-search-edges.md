---
id: T19
title: '[C26] UC: Soft-expired edge excluded from search_edges'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:52.626516Z'
updated_at: '2026-04-19T02:04:38.226335Z'
tags:
- uc
- test
- search
---

<!-- DESCRIPTION -->
Fixed in PR #8 (commit 51f1440, merged 2026-04-18). Soft-expire filtering now covers search_entities, search_edges, fts_search, get_1hop_candidates, and cleanup_expired. All paths filter on COALESCE(json_extract(metadata, '$.marked_for_deletion'), 0) = 0.
