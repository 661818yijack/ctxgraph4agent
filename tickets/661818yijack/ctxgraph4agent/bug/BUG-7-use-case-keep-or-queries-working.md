---
id: BUG-7
title: 'Use case: Keep OR queries working'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-13T02:02:59.648823Z'
updated_at: '2026-09-13T02:05:15.111474Z'
tags:
- cron-2026-09-13
- fts-hyphen
- use-case
- parent:bug-1
- requirement:bug-6
version: 2
schema_version: v0.2
due_date: '2026-09-14'
---

<!-- DESCRIPTION -->
billing OR discount should still match either term. Requirement: BUG-6. Parent: BUG-1

<!-- FIX -->
Implemented and verified in branch fix/fts-hyphen-literals-20260913: quote hyphenated/punctuated FTS tokens, add regression tests for episode and entity searches, preserve boolean searches. Verification: cargo test -p ctxgraph --lib --tests passed.
