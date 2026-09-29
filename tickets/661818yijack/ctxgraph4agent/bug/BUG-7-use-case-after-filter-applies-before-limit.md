---
id: BUG-7
title: 'Use case: after filter applies before limit'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:15.891713Z'
updated_at: '2026-09-29T02:06:11.099841Z'
tags:
- parent:query-after-filter
- req:storage-after
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
Older high-ranking matches cannot consume the limit and hide eligible recent matches.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
