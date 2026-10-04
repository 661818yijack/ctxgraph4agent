---
id: BUG-5
title: 'Use case: GLM returns numeric label id'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:47.307714Z'
updated_at: '2026-10-04T02:06:20.441374Z'
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
Use case for BUG-2: an LLM response item like {id: 1, label: ...} should map to the first candidate.

<!-- FIX -->
Covered by parses_numeric_label_ids unit test.
