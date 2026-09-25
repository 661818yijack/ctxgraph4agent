---
id: ENHANCEMENT-2
title: 'stats json req: CLI exposes explicit format option'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:21.252368Z'
updated_at: '2026-09-25T05:37:21.411220Z'
tags:
- cron-20260925
- stats
- json
- requirement
version: 2
schema_version: v0.2
due_date: '2026-09-26'
---

<!-- DESCRIPTION -->
Requirement: ctxgraph stats must accept a format option with text as the default and json as the new machine-readable mode.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
