---
id: T15
title: '[C23] UC: Edges for soft-expired entity excluded from get_edges_for_entity'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-17T02:05:09.165439Z'
updated_at: '2026-04-18T02:03:19.999441Z'
tags:
- uc
- c23
- test
---

<!-- DESCRIPTION -->
Given an entity has edges AND the entity is soft-expired, When get_edges_for_entity(entity_id) is called, Then no edges are returned for the soft-expired entity.
