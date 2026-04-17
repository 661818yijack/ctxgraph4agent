---
id: T11
title: '[C19] UC: cleanup_expired does not delete experiences younger than 6 months'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-15T02:05:03.759878Z'
updated_at: '2026-04-15T02:10:32.265407Z'
tags:
- uc
- test
- cleanup
- experience
---

<!-- DESCRIPTION -->
Given an Experience entity created 100 days ago, when cleanup_expired runs with default grace period, then the entity is NOT deleted. Given an Experience entity created 200 days ago, when cleanup runs, then it IS deleted.
