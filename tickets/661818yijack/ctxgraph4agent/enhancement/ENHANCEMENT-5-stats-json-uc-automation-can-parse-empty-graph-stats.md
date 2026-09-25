---
id: ENHANCEMENT-5
title: 'stats json uc: automation can parse empty graph stats'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:21.881860Z'
updated_at: '2026-09-25T05:37:21.837478Z'
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
Use case: a shell script can run ctxgraph stats --format json on a new graph and parse valid JSON counts without scraping text.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
