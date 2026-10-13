---
id: BUG-5
title: 'UC1.3: Short non-ASCII stale content is unchanged'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: null
created_at: '2026-10-13T02:01:51.589518Z'
updated_at: '2026-10-13T02:04:30.673436Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-14'
---

<!-- DESCRIPTION -->
Use case: short non-ASCII content displays exactly, without unnecessary ellipsis.

<!-- FIX -->
Added short non-ASCII value test; unshortened values are preserved.
