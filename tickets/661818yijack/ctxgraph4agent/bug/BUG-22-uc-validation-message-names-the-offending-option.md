---
id: BUG-22
title: 'UC: validation message names the offending option'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-12T02:02:08.218826Z'
updated_at: '2026-10-12T02:04:59.466165Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-13'
---

<!-- DESCRIPTION -->
Use case under BUG-14: validation error mentions --limit and zero.

<!-- FIX -->
Shared validator reports: --limit must be greater than 0.
