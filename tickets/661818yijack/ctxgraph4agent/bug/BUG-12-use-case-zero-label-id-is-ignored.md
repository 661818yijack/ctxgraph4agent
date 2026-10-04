---
id: BUG-12
title: 'Use case: zero label id is ignored'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:03:03.464339Z'
updated_at: '2026-10-04T02:06:27.625293Z'
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
Use case for BUG-4: id 0 should not map through underflow or attach a label to any candidate.

<!-- FIX -->
Covered by ignores_invalid_label_ids unit test for id 0.
