---
id: ENHANCEMENT-4
title: 'stats json req: keep human stats output unchanged'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: low
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:21.666216Z'
updated_at: '2026-09-25T05:37:21.696936Z'
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
Requirement: existing ctxgraph stats text output remains the default for humans and keeps the same labels for current users.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
