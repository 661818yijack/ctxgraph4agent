---
id: BUG-3
title: 'UC1.1: Long Chinese stale content displays with ellipsis'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-13T02:01:51.303894Z'
updated_at: '2026-10-13T02:04:30.382189Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-14'
---

<!-- DESCRIPTION -->
Use case: long Chinese stale memory content prints a shortened preview ending in ellipsis.

<!-- FIX -->
Added Chinese content regression test; table printing no longer panics.
