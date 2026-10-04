---
id: BUG-3
title: Require string batch label IDs to keep working
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:38.505800Z'
updated_at: '2026-10-04T02:06:13.369503Z'
tags:
- requirement
- learn
- llm-parser
version: 2
schema_version: v0.2
due_date: '2026-10-05'
---

<!-- DESCRIPTION -->
Requirement: existing string JSON id values continue to map back to 1-based candidate positions.

<!-- FIX -->
String JSON id values continue to parse through usize::parse.
