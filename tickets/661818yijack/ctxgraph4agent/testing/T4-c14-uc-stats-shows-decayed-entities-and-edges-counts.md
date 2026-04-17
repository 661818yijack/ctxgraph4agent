---
id: T4
title: '[C14] UC: Stats shows decayed entities and edges counts'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-13T02:05:29.377226Z'
updated_at: '2026-04-13T02:09:38.682105Z'
tags:
- uc
- stats
version: 2
---

<!-- DESCRIPTION -->
GIVEN a graph with some entities past TTL+grace, WHEN ctxgraph stats is run, THEN decayed_entities and decayed_edges counts are displayed in a Memory Health section.
