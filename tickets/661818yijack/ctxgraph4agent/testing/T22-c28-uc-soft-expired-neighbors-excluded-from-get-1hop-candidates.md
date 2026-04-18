---
id: T22
title: '[C28] UC: Soft-expired neighbors excluded from get_1hop_candidates'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:53.109306Z'
updated_at: '2026-04-18T02:03:53.109306Z'
tags:
- uc
- test
- retrieval
- graph
---

<!-- DESCRIPTION -->
Given an entity with soft-expired neighbor entities/edges, When get_1hop_candidates() is called, Then soft-expired neighbors are skipped and do not become RetrievalCandidates.
