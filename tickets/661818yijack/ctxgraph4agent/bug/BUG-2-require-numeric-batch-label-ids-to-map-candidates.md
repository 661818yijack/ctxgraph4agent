---
id: BUG-2
title: Require numeric batch label IDs to map candidates
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:38.368794Z'
updated_at: '2026-10-04T02:06:13.234975Z'
tags:
- requirement
- learn
- llm-parser
version: 2
schema_version: v0.2
due_date: '2026-10-05'
---

<!-- DESCRIPTION -->
Requirement: numeric JSON id values from LLM output map back to 1-based candidate positions.

<!-- FIX -->
Numeric JSON id values are converted to usize before candidate mapping.
