---
id: T28
title: '[C32] UC: Empty collections serialize as empty objects not empty arrays'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-19T02:05:11.845182Z'
updated_at: '2026-04-19T02:07:44.905149Z'
tags:
- uc
- test
- stats
- edge-case
---

<!-- DESCRIPTION -->
Given stats has no data for a field (empty Vec), When stats() is called, Then the field must be an empty object {} not an empty array [].
