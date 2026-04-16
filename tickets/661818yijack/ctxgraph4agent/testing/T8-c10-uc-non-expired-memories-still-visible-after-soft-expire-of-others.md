---
id: T8
title: '[C10] UC: Non-expired memories still visible after soft-expire of others'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-16T02:05:34.593003Z'
updated_at: '2026-04-16T02:05:34.593003Z'
tags:
- uc
- test
- search
- isolation
---

<!-- DESCRIPTION -->
Given entity A is marked_for_deletion=true AND entity B is NOT marked
When fts_search_entities() is called with a query matching both
Then only entity B appears in results (A is filtered, B is not)
