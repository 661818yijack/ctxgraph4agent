---
id: T27
title: '[C31] UC: decayed_entities_by_type returned as JSON object with string keys'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-19T02:05:10.580630Z'
updated_at: '2026-04-19T02:07:44.742334Z'
tags:
- uc
- test
- stats
---

<!-- DESCRIPTION -->
Given stats has decayed entity data, When stats() is called, Then decayed_entities_by_type must be a JSON object {type: count} not an array.
