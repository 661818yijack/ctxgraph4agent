---
id: BUG-19
title: 'REQ: Validate zero limits for reverify list pagination'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-12T02:02:07.808992Z'
updated_at: '2026-10-12T02:04:53.980176Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-13'
---

<!-- DESCRIPTION -->
Requirement under BUG-14: reverify list must reject --limit 0 with a clear option-specific error.

<!-- FIX -->
Added the same positive-limit validation to reverify list pagination.
