---
id: BUG-13
title: 'Use case: non-numeric label id is ignored'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:03:03.593303Z'
updated_at: '2026-10-04T02:06:27.760461Z'
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
Use case for BUG-4: missing, object, or non-numeric string ids are ignored safely.

<!-- FIX -->
Covered by ignores_invalid_label_ids unit test for non-numeric and missing ids.
