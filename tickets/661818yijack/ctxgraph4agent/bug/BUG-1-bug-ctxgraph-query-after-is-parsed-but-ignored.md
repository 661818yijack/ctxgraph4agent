---
id: BUG-1
title: 'Bug: ctxgraph query --after is parsed but ignored'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:15.067544Z'
updated_at: '2026-09-29T02:06:10.203398Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
Parent: make ctxgraph query honor its documented --after ISO-8601 filter instead of returning older matching episodes.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
