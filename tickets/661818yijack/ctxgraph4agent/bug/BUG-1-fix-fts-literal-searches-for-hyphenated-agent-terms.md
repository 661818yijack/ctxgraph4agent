---
id: BUG-1
title: Fix FTS literal searches for hyphenated agent terms
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-13T02:02:38.406526Z'
updated_at: '2026-09-13T02:05:14.103825Z'
tags:
- cron-2026-09-13
- fts-hyphen
- parent
version: 2
schema_version: v0.2
due_date: '2026-09-14'
---

<!-- DESCRIPTION -->
Parent: FTS5 MATCH should handle real agent terms like ctxgraph-mcp, Node.js, and user punctuation without SQL/FTS syntax errors while preserving useful boolean search.

<!-- FIX -->
Implemented and verified in branch fix/fts-hyphen-literals-20260913: quote hyphenated/punctuated FTS tokens, add regression tests for episode and entity searches, preserve boolean searches. Verification: cargo test -p ctxgraph --lib --tests passed.
