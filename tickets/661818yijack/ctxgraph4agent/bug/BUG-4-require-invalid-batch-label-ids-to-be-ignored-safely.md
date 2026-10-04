---
id: BUG-4
title: Require invalid batch label IDs to be ignored safely
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:38.639751Z'
updated_at: '2026-10-04T02:06:13.504311Z'
tags:
- requirement
- learn
- llm-parser
version: 2
schema_version: v0.2
due_date: '2026-10-05'
---

<!-- DESCRIPTION -->
Requirement: out-of-range, zero, non-numeric, and missing ids are ignored without creating wrong labels.

<!-- FIX -->
Zero, out-of-range, non-numeric, and missing ids produce no candidate mapping.
