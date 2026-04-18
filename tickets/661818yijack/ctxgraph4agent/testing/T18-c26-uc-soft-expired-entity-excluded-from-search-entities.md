---
id: T18
title: '[C26] UC: Soft-expired entity excluded from search_entities'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:45.904310Z'
updated_at: '2026-04-18T02:03:45.904310Z'
tags:
- uc
- test
- search
---

<!-- DESCRIPTION -->
Given an entity exists in the graph AND has been marked_for_deletion=true via forget(hard=false), When search_entities() is called with a query matching that entity, Then the entity must NOT appear in the results.
