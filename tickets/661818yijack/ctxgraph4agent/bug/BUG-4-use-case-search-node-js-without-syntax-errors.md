---
id: BUG-4
title: 'Use case: Search Node.js without syntax errors'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-13T02:02:59.136704Z'
updated_at: '2026-09-13T02:05:14.606801Z'
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
A dotted technology name should stay a literal FTS term. Requirement: BUG-2. Parent: BUG-1

<!-- FIX -->
Implemented and verified in branch fix/fts-hyphen-literals-20260913: quote hyphenated/punctuated FTS tokens, add regression tests for episode and entity searches, preserve boolean searches. Verification: cargo test -p ctxgraph --lib --tests passed.
