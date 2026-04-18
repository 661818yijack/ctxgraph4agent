---
id: T24
title: '[C29] UC: Cleanup sweep handles FK safety when deleting soft-expired entities'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-18T02:03:53.429997Z'
updated_at: '2026-04-18T02:03:53.429997Z'
tags:
- uc
- test
- cleanup
- fk-safety
---

<!-- DESCRIPTION -->
Given a soft-expired entity has edges pointing to/from it, When cleanup_expired() deletes the entity, Then related edges and episode_entities junction rows are also cleaned up without FK errors.
