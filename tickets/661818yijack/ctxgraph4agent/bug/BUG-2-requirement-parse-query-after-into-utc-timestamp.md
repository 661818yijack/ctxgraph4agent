---
id: BUG-2
title: 'Requirement: parse query --after into UTC timestamp'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:15.209689Z'
updated_at: '2026-09-29T02:06:10.352712Z'
tags:
- parent:query-after-filter
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
Convert valid RFC3339 --after values before search and preserve existing no-filter behavior when omitted.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
