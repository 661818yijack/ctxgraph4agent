---
id: BUG-13
title: 'Use case: targeted tests run in cron reliably'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:16.726578Z'
updated_at: '2026-09-29T02:06:11.956503Z'
tags:
- parent:query-after-filter
- req:tests
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
The focused cargo test command completes within the daily job budget.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
