---
id: T21
title: '[C27] UC: Soft-expired items excluded from fts_search retrieval pipeline'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:52.948406Z'
updated_at: '2026-04-18T02:03:52.948406Z'
tags:
- uc
- test
- retrieval
- fts5
---

<!-- DESCRIPTION -->
Given entities and edges exist with marked_for_deletion=true, When retrieve_for_context() runs the FTS5 search phase, Then no marked_for_deletion items appear in the candidate list from fts_search_entities/edges/episodes.
