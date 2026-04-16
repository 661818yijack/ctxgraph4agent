---
id: T4
title: '[C10] UC: Soft-expired entity excluded from FTS5 entity search'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-16T02:05:20.363748Z'
updated_at: '2026-04-16T02:05:20.363748Z'
tags:
- uc
- test
- search
- entity
---

<!-- DESCRIPTION -->
Given an entity exists in the graph AND has been marked_for_deletion=true via forget(hard=false)
When fts_search_entities() is called with a query matching that entity
Then the entity must NOT appear in the results.
