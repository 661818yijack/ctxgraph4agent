---
id: T26
title: '[C31] UC: total_entities_by_type returned as JSON object with string keys'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-19T02:05:09.309905Z'
updated_at: '2026-04-19T02:05:09.309905Z'
tags:
- uc
- test
- stats
---

<!-- DESCRIPTION -->
Given stats has entity type data, When stats() is called, Then total_entities_by_type must be a JSON object {type: count} not an array.
