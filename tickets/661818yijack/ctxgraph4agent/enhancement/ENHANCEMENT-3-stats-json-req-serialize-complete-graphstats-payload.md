---
id: ENHANCEMENT-3
title: 'stats json req: serialize complete GraphStats payload'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:21.466647Z'
updated_at: '2026-09-25T05:37:21.555095Z'
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
Requirement: JSON stats output must include all GraphStats fields, including cleanup visibility and memory type breakdowns.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
