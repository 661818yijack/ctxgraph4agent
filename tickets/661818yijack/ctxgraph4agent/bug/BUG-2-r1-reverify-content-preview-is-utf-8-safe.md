---
id: BUG-2
title: 'R1: Reverify content preview is UTF-8 safe'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-13T02:01:51.160520Z'
updated_at: '2026-10-13T02:04:30.235107Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-14'
---

<!-- DESCRIPTION -->
Requirement under parent: long non-ASCII stale memory content must shorten without panicking or breaking character boundaries.

<!-- FIX -->
Content preview now uses chars rather than byte indexes.
