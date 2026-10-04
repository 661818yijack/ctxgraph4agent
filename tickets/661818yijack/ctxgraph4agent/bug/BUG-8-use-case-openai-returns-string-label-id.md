---
id: BUG-8
title: 'Use case: OpenAI returns string label id'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:51.955686Z'
updated_at: '2026-10-04T02:06:20.847199Z'
tags:
- use-case
- bug-3
- learn
- llm-parser
version: 2
schema_version: v0.2
due_date: '2026-10-05'
---

<!-- DESCRIPTION -->
Use case for BUG-3: an LLM response item like {id: "1", label: ...} should keep mapping to the first candidate.

<!-- FIX -->
Covered by parses_string_label_ids unit test.
