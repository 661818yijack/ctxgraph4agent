---
id: T6
title: '[C14] UC: Stats shows entities breakdown by memory type'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-13T02:05:32.016217Z'
updated_at: '2026-04-13T02:09:41.227875Z'
tags:
- uc
- stats
- types
version: 2
---

<!-- DESCRIPTION -->
GIVEN a graph with entities of multiple memory types, WHEN ctxgraph stats is run, THEN total_entities_by_type is displayed as a breakdown (e.g., Fact: 5, Experience: 3, Pattern: 2). decayed_entities_by_type is also shown if any are decayed.
