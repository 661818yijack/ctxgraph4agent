---
id: T20
title: '[C26] UC: Non-expired memories still visible after soft-expire of others in
  search'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:52.785539Z'
updated_at: '2026-04-18T02:03:52.785539Z'
tags:
- uc
- test
- search
- isolation
---

<!-- DESCRIPTION -->
Given entity A is marked_for_deletion=true AND entity B is NOT marked, When search_entities() is called with a query matching both, Then only entity B appears (A is filtered, B is not).
