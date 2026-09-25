---
id: ENHANCEMENT-13
title: 'stats json uc: JSON formatting errors propagate as command failures'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: low
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:23.434651Z'
updated_at: '2026-09-25T05:37:22.963708Z'
tags:
- cron-20260925
- stats
- json
- use-case
version: 2
schema_version: v0.2
due_date: '2026-09-26'
---

<!-- DESCRIPTION -->
Use case: if serialization unexpectedly fails, ctxgraph stats exits with an error rather than printing malformed JSON.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
