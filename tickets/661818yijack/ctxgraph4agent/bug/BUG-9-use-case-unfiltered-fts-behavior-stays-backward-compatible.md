---
id: BUG-9
title: 'Use case: unfiltered FTS behavior stays backward-compatible'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:16.160843Z'
updated_at: '2026-09-29T02:06:11.383919Z'
tags:
- parent:query-after-filter
- req:storage-after
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
Existing callers with no source or date filter receive the same ranked results.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
