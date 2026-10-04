---
id: BUG-7
title: 'Use case: numeric ID does not become empty string'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:47.565509Z'
updated_at: '2026-10-04T02:06:20.713946Z'
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
Use case for BUG-2: a numeric id must be converted to its decimal string before parsing, not discarded as an empty id.

<!-- FIX -->
Numeric ids are converted with as_u64 and usize::try_from instead of becoming empty strings.
