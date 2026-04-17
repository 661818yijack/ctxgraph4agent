---
id: T13
title: '[C21] UC: all existing tests pass after TTL change'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-15T02:05:13.024919Z'
updated_at: '2026-04-15T02:10:32.604950Z'
tags:
- uc
- test
- regression
---

<!-- DESCRIPTION -->
Given the full test suite, when the experience TTL is changed from 14d to 180d, then all 272+ tests still pass. Specifically test_decay_experience_at_ttl_scores_zero must be updated to use 180d, and any cleanup tests that expect 14d deletion must be updated.
