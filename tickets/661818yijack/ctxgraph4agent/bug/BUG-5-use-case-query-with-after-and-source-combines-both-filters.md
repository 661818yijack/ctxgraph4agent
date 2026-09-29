---
id: BUG-5
title: 'Use case: query with --after and source combines both filters'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:15.617843Z'
updated_at: '2026-09-29T02:06:10.807009Z'
tags:
- parent:query-after-filter
- req:parse-after
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
A user scopes search by both timestamp and source; both constraints are applied together.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
