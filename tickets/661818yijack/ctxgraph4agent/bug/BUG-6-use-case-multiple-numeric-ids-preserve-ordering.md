---
id: BUG-6
title: 'Use case: multiple numeric IDs preserve ordering'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:47.435898Z'
updated_at: '2026-10-04T02:06:20.579092Z'
tags:
- use-case
- bug-2
- learn
- llm-parser
version: 2
schema_version: v0.2
due_date: '2026-10-05'
---

<!-- DESCRIPTION -->
Use case for BUG-2: numeric ids 1 and 2 should map to the matching candidates even when both appear in one array.

<!-- FIX -->
Covered by parses_numeric_label_ids unit test with two numeric ids.
