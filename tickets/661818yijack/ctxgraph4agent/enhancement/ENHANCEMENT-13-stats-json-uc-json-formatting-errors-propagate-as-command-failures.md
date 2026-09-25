---
id: ENHANCEMENT-13
title: 'stats json uc: JSON formatting errors propagate as command failures'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: low
status: open
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:23.434651Z'
updated_at: '2026-09-25T05:32:23.434651Z'
tags:
- cron-20260925
- stats
- json
- use-case
version: 1
schema_version: v0.2
due_date: '2026-09-26'
---

<!-- DESCRIPTION -->
Use case: if serialization unexpectedly fails, ctxgraph stats exits with an error rather than printing malformed JSON.
