---
id: BUG-8
title: 'Use case: source-only FTS behavior stays backward-compatible'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:16.026523Z'
updated_at: '2026-09-29T02:06:11.241708Z'
tags:
- parent:query-after-filter
- req:storage-after
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
Existing callers using only source filtering receive the same scoped results.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
