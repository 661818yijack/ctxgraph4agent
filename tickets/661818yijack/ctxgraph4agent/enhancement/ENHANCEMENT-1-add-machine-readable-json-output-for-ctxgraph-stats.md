---
id: ENHANCEMENT-1
title: Add machine-readable JSON output for ctxgraph stats
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:21.038097Z'
updated_at: '2026-09-25T05:37:21.267578Z'
tags:
- cron-20260925
- stats
- json
- parent
version: 2
schema_version: v0.2
due_date: '2026-09-26'
---

<!-- DESCRIPTION -->
Parent: ctxgraph stats currently only renders human text, making automation and MCP-adjacent health checks parse terminal formatting. Add an explicit JSON format while preserving text defaults.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
