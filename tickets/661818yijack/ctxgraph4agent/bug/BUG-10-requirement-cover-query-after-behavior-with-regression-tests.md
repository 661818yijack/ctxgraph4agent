---
id: BUG-10
title: 'Requirement: cover query after behavior with regression tests'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:16.297301Z'
updated_at: '2026-09-29T02:06:11.525424Z'
tags:
- parent:query-after-filter
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
Add tests proving --after/search-after behavior, combined source/date filtering, and backward compatibility.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
