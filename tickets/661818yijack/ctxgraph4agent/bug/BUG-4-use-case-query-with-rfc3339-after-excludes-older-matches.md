---
id: BUG-4
title: 'Use case: query with RFC3339 --after excludes older matches'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:15.481433Z'
updated_at: '2026-09-29T02:06:10.656512Z'
tags:
- parent:query-after-filter
- req:parse-after
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
A user investigates recent decisions and old matching episodes before the cutoff are not shown.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
