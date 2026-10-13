---
id: BUG-9
title: 'UC2.3: ASCII IDs keep existing 8-character preview'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: null
created_at: '2026-10-13T02:01:52.165784Z'
updated_at: '2026-10-13T02:04:31.250918Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-14'
---

<!-- DESCRIPTION -->
Use case: existing long ASCII IDs continue to show the first eight visible characters plus ellipsis.

<!-- FIX -->
Added ASCII compatibility assertion preserving first eight visible characters plus ellipsis.
