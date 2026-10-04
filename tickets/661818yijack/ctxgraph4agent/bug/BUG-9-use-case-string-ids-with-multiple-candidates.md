---
id: BUG-9
title: 'Use case: string IDs with multiple candidates'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:52.086737Z'
updated_at: '2026-10-04T02:06:20.983424Z'
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
Use case for BUG-3: string ids "1" and "2" map to their corresponding candidates in one batch response.

<!-- FIX -->
Covered by parses_string_label_ids unit test with two string ids.
