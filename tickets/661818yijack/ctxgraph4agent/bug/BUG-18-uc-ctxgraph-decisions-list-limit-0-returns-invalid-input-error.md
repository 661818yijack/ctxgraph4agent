---
id: BUG-18
title: 'UC: ctxgraph decisions list --limit 0 returns invalid input error'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-12T02:01:59.953535Z'
updated_at: '2026-10-12T02:04:53.828818Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-13'
---

<!-- DESCRIPTION -->
Use case under BUG-14: decisions list with --limit 0 fails clearly.

<!-- FIX -->
ctxgraph decisions list --limit 0 now fails with invalid input before opening the graph.
