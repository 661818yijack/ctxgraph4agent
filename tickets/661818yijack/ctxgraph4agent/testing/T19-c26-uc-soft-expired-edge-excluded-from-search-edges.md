---
id: T19
title: '[C26] UC: Soft-expired edge excluded from search_edges'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:52.626516Z'
updated_at: '2026-04-18T02:03:52.626516Z'
tags:
- uc
- test
- search
---

<!-- DESCRIPTION -->
Given an edge exists AND has been marked_for_deletion=true, When search_edges() is called with a matching query, Then the edge must NOT appear in results.
