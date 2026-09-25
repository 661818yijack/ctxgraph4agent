---
id: ENHANCEMENT-6
title: 'stats json uc: automation can read cleanup fields'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:22.099294Z'
updated_at: '2026-09-25T05:37:21.976879Z'
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
Use case: monitoring can read cleanup_interval, queries_since_cleanup, last_cleanup_at, and cleanup_in_progress directly from JSON.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
