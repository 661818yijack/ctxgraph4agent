---
id: BUG-6
title: 'Requirement: Preserve current boolean query behavior'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-13T02:02:59.477107Z'
updated_at: '2026-09-13T02:05:14.940273Z'
tags:
- cron-2026-09-13
- fts-hyphen
- requirement
- parent:bug-1
version: 2
schema_version: v0.2
due_date: '2026-09-14'
---

<!-- DESCRIPTION -->
Keep existing AND/OR/NOT usage working for advanced searches already covered by tests. Parent: BUG-1

<!-- FIX -->
Implemented and verified in branch fix/fts-hyphen-literals-20260913: quote hyphenated/punctuated FTS tokens, add regression tests for episode and entity searches, preserve boolean searches. Verification: cargo test -p ctxgraph --lib --tests passed.
