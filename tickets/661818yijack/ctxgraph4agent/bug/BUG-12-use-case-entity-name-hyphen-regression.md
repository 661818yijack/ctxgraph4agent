---
id: BUG-12
title: 'Use case: Entity name hyphen regression'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-13T02:03:00.491485Z'
updated_at: '2026-09-13T02:05:15.957021Z'
tags:
- cron-2026-09-13
- fts-hyphen
- use-case
- parent:bug-1
- requirement:bug-10
version: 2
schema_version: v0.2
due_date: '2026-09-14'
---

<!-- DESCRIPTION -->
Entity search covers ctxgraph-mcp names. Requirement: BUG-10. Parent: BUG-1

<!-- FIX -->
Implemented and verified in branch fix/fts-hyphen-literals-20260913: quote hyphenated/punctuated FTS tokens, add regression tests for episode and entity searches, preserve boolean searches. Verification: cargo test -p ctxgraph --lib --tests passed.
