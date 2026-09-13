---
id: BUG-10
title: 'Requirement: Cover episode and entity FTS paths'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-13T02:03:00.152283Z'
updated_at: '2026-09-13T02:05:15.623585Z'
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
Regression tests must cover both episode content search and entity name search. Parent: BUG-1

<!-- FIX -->
Implemented and verified in branch fix/fts-hyphen-literals-20260913: quote hyphenated/punctuated FTS tokens, add regression tests for episode and entity searches, preserve boolean searches. Verification: cargo test -p ctxgraph --lib --tests passed.
