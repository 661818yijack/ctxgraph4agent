---
id: BUG-6
title: 'R2: Reverify ID preview is UTF-8 safe'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-13T02:01:51.736640Z'
updated_at: '2026-10-13T02:04:30.820229Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-14'
---

<!-- DESCRIPTION -->
Requirement under parent: stale memory IDs should be shortened by characters, not byte offsets, so imported/custom non-ASCII IDs cannot panic the table.

<!-- FIX -->
ID preview now uses chars rather than byte indexes.
