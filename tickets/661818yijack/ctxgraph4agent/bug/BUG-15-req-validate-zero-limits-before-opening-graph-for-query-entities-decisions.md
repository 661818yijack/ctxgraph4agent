---
id: BUG-15
title: 'REQ: Validate zero limits before opening graph for query/entities/decisions'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-12T02:01:59.543799Z'
updated_at: '2026-10-12T02:04:50.067809Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-13'
---

<!-- DESCRIPTION -->
Requirement under BUG-14: query, entities list, and decisions list must reject --limit 0 before graph access.

<!-- FIX -->
Added shared positive-limit validation to query, entities list, and decisions list.
