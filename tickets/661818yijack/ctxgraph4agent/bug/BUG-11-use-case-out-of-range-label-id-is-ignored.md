---
id: BUG-11
title: 'Use case: out-of-range label id is ignored'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:03:03.331530Z'
updated_at: '2026-10-04T02:06:27.493333Z'
tags:
- use-case
- bug-4
- learn
- llm-parser
version: 2
schema_version: v0.2
due_date: '2026-10-05'
---

<!-- DESCRIPTION -->
Use case for BUG-4: id values above the candidate count should not create labels for non-existent candidates.

<!-- FIX -->
Covered by ignores_invalid_label_ids unit test for id above candidate count.
