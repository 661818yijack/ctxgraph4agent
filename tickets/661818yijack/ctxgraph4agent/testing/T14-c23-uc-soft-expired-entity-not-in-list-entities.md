---
id: T14
title: '[C23] UC: Soft-expired entity not in list_entities'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-17T02:05:07.899329Z'
updated_at: '2026-04-18T02:03:19.833731Z'
tags:
- uc
- c23
- test
---

<!-- DESCRIPTION -->
Given an entity exists AND forget(id, hard=false) is called, When list_entities() is called, Then the soft-expired entity does not appear in results.
