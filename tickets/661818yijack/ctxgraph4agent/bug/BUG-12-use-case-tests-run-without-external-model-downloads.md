---
id: BUG-12
title: 'Use case: tests run without external model downloads'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:16.581342Z'
updated_at: '2026-09-29T02:06:11.806611Z'
tags:
- parent:query-after-filter
- req:tests
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
Regression tests use core storage/FTS and do not require ONNX models or cloud API keys.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
