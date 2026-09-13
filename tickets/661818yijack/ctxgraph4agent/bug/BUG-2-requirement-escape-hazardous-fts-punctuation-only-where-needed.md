---
id: BUG-2
title: 'Requirement: Escape hazardous FTS punctuation only where needed'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-13T02:02:38.579719Z'
updated_at: '2026-09-13T02:05:14.270352Z'
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
Ensure hyphens and punctuation are treated as literals unless part of supported boolean syntax. Parent: BUG-1

<!-- FIX -->
Implemented and verified in branch fix/fts-hyphen-literals-20260913: quote hyphenated/punctuated FTS tokens, add regression tests for episode and entity searches, preserve boolean searches. Verification: cargo test -p ctxgraph --lib --tests passed.
