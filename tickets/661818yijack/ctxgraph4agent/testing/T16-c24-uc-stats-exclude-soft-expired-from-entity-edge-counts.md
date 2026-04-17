---
id: T16
title: '[C24] UC: Stats exclude soft-expired from entity/edge counts'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-17T02:05:10.432099Z'
updated_at: '2026-04-17T02:05:10.432099Z'
tags:
- uc
- c24
- test
---

<!-- DESCRIPTION -->
Given 3 entities exist AND 1 is soft-expired, When stats() is called, Then entity_count = 2 (not 3). Same for edge_count.
