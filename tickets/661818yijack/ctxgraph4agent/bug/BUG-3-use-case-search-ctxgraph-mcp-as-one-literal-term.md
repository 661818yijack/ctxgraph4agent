---
id: BUG-3
title: 'Use case: Search ctxgraph-mcp as one literal term'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-13T02:02:38.762005Z'
updated_at: '2026-09-13T02:05:14.436967Z'
tags:
- cron-2026-09-13
- fts-hyphen
- use-case
- parent:bug-1
- requirement:bug-2
version: 2
schema_version: v0.2
due_date: '2026-09-14'
---

<!-- DESCRIPTION -->
A query for ctxgraph-mcp should return matching episodes instead of being parsed as FTS syntax. Requirement: BUG-2. Parent: BUG-1

<!-- FIX -->
Implemented and verified in branch fix/fts-hyphen-literals-20260913: quote hyphenated/punctuated FTS tokens, add regression tests for episode and entity searches, preserve boolean searches. Verification: cargo test -p ctxgraph --lib --tests passed.
