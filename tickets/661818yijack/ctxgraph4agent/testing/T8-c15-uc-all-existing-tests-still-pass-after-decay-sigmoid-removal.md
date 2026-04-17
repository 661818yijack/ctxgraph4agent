---
id: T8
title: '[C15] UC: All existing tests still pass after decay_sigmoid removal'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-13T02:05:35.829086Z'
updated_at: '2026-04-13T02:09:43.775119Z'
tags:
- uc
- tests
- dead-code
version: 2
---

<!-- DESCRIPTION -->
GIVEN decay_sigmoid is removed, WHEN cargo test is run, THEN all 276 existing tests pass with no failures.
