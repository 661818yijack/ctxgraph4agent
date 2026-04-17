---
id: T17
title: '[C24] UC: Entity type counts exclude soft-expired'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-17T02:05:11.718662Z'
updated_at: '2026-04-17T02:05:11.718662Z'
tags:
- uc
- c24
- test
---

<!-- DESCRIPTION -->
Given 2 fact entities and 1 soft-expired fact entity, When get_entity_counts_by_type() is called, Then fact count = 2 (not 3).
