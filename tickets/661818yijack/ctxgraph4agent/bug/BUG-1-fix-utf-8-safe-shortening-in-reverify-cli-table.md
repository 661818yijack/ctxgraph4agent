---
id: BUG-1
title: Fix UTF-8-safe shortening in reverify CLI table
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-13T02:01:51.011029Z'
updated_at: '2026-10-13T02:04:30.090266Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-14'
---

<!-- DESCRIPTION -->
Parent: ctxgraph reverify list formats stale memory IDs and content for terminal output. Byte slicing can panic on multibyte UTF-8 when shortening. Fix only the CLI table shortening path we use, with tests first. Requirements: R1 UTF-8 safe content preview, R2 UTF-8 safe ID preview, R3 preserve existing ASCII layout.

<!-- FIX -->
Added character-boundary-safe preview helper for reverify CLI table and tests covering non-ASCII IDs/content plus ASCII compatibility.
