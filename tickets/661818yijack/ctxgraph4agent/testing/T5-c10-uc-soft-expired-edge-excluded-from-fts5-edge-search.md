---
id: T5
title: '[C10] UC: Soft-expired edge excluded from FTS5 edge search'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-16T02:05:21.607222Z'
updated_at: '2026-04-16T02:05:21.607222Z'
tags:
- uc
- test
- search
- edge
---

<!-- DESCRIPTION -->
Given an edge exists in the graph AND has been marked_for_deletion=true via forget(hard=false)
When fts_search_edges() is called with a query matching that edge
Then the edge must NOT appear in the results.
