---
id: ENHANCEMENT-6
title: 'REQ: Empty entity output must avoid stale version promises'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-03T02:03:25.291299Z'
updated_at: '2026-10-03T02:05:42.514860Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-04'
---

<!-- DESCRIPTION -->
Requirement: CLI output must not mention v0.2 or future auto-extraction promises that are stale for the v0.8 codebase.

<!-- FIX -->
Removed stale v0.2 future-release text from entities empty-state output.
