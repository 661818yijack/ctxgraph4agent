---
id: ENHANCEMENT-11
title: 'stats json uc: invalid format is rejected by CLI parser'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:23.045516Z'
updated_at: '2026-09-25T05:37:22.680652Z'
tags:
- cron-20260925
- stats
- cli
- use-case
version: 2
schema_version: v0.2
due_date: '2026-09-26'
---

<!-- DESCRIPTION -->
Use case: users get a parser error for unsupported stats formats instead of silent fallback to surprising output.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
