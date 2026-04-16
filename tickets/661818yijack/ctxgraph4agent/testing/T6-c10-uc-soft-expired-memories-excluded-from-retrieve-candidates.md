---
id: T6
title: '[C10] UC: Soft-expired memories excluded from retrieve_candidates'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-16T02:05:22.868481Z'
updated_at: '2026-04-16T02:05:22.868481Z'
tags:
- uc
- test
- retrieval
- candidates
---

<!-- DESCRIPTION -->
Given entities and edges exist in the graph AND some have been marked_for_deletion=true
When retrieve_candidates() is called
Then no marked_for_deletion items appear in the candidates
