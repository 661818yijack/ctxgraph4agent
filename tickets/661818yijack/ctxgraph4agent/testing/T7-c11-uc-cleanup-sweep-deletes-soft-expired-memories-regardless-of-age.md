---
id: T7
title: '[C11] UC: Cleanup sweep deletes soft-expired memories regardless of age'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-16T02:05:33.324022Z'
updated_at: '2026-04-16T02:05:33.324022Z'
tags:
- uc
- test
- cleanup
- sweep
---

<!-- DESCRIPTION -->
Given entities and edges exist with marked_for_deletion=true AND they are freshly created (well within TTL)
When cleanup_expired() runs
Then those marked_for_deletion items are deleted (not just archived)
And the CleanupResult includes the counts
