---
id: ENHANCEMENT-9
title: 'stats json uc: default text output still supports humans'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-25T05:32:22.686720Z'
updated_at: '2026-09-25T05:37:22.399771Z'
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
Use case: a human running ctxgraph stats with no flags still sees the familiar labeled report.

<!-- FIX -->
Implemented in commit ba89a08: ctxgraph stats now supports --format text|json, serializes the complete GraphStats payload as compact JSON, preserves text output, and is covered by tests plus manual CLI verification.
