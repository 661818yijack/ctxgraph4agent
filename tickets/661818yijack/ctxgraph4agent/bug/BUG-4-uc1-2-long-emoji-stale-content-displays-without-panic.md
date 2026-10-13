---
id: BUG-4
title: 'UC1.2: Long emoji stale content displays without panic'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-13T02:01:51.446113Z'
updated_at: '2026-10-13T02:04:30.527083Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-14'
---

<!-- DESCRIPTION -->
Use case: multi-byte emoji around the preview boundary does not panic table formatting.

<!-- FIX -->
Added emoji content regression test; table printing no longer panics.
