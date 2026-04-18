---
id: T23
title: '[C29] UC: Cleanup sweep deletes soft-expired memories regardless of age'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:53.270615Z'
updated_at: '2026-04-18T02:03:53.270615Z'
tags:
- uc
- test
- cleanup
---

<!-- DESCRIPTION -->
Given entities and edges with marked_for_deletion=true AND they are freshly created (well within TTL), When cleanup_expired() runs, Then those items are deleted and CleanupResult includes the counts.
