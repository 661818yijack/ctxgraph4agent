---
id: BUG-14
title: 'BUG: CLI zero limits produce misleading empty results instead of validation
  errors'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-12T02:01:55.697597Z'
updated_at: '2026-10-12T02:04:49.917154Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-13'
---

<!-- DESCRIPTION -->
Daily cron ticket tree for bug fix: reject --limit 0 in user-facing CLI commands so users get a clear validation error instead of misleading empty output.

<!-- FIX -->
Validated --limit values before CLI graph access; zero limits now return invalid input.
