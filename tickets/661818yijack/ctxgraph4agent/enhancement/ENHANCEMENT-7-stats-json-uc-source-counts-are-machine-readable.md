---
id: ENHANCEMENT-7
title: 'stats json uc: source counts are machine-readable'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:22.305135Z'
updated_at: '2026-09-25T05:37:22.117461Z'
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
Use case: automation can read source count pairs from stats JSON to see which sources dominate the graph.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
