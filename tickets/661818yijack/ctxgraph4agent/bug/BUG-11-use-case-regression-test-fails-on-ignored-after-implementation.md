---
id: BUG-11
title: 'Use case: regression test fails on ignored-after implementation'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:16.439690Z'
updated_at: '2026-09-29T02:06:11.665216Z'
tags:
- parent:query-after-filter
- req:tests
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
A test dataset with old/new matching episodes catches the previous ignored --after bug.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
