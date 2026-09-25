---
id: ENHANCEMENT-10
title: 'stats json uc: text format can be requested explicitly'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: low
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:22.869760Z'
updated_at: '2026-09-25T05:37:22.539068Z'
tags:
- cron-20260925
- stats
- text
- use-case
version: 2
schema_version: v0.2
due_date: '2026-09-26'
---

<!-- DESCRIPTION -->
Use case: scripts or docs can use ctxgraph stats --format text to document the default output explicitly.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
