---
id: T5
title: '[C14] UC: Stats shows cleanup status section'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-13T02:05:30.730211Z'
updated_at: '2026-04-13T02:09:39.974987Z'
tags:
- uc
- stats
- cleanup
version: 2
---

<!-- DESCRIPTION -->
GIVEN a graph that has run cleanup before, WHEN ctxgraph stats is run, THEN last_cleanup_at, cleanup_interval, queries_since_cleanup are displayed. If cleanup never ran, show 'never'. If cleanup_in_progress is true, show a warning.
