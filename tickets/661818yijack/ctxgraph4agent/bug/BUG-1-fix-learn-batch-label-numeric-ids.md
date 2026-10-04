---
id: BUG-1
title: Fix learn batch label numeric IDs
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:38.234914Z'
updated_at: '2026-10-04T02:06:13.098296Z'
tags:
- parent
- learn
- llm-parser
version: 2
schema_version: v0.2
due_date: '2026-10-05'
---

<!-- DESCRIPTION -->
Parent: ctxgraph learn should accept numeric and string ids in LLM batch label JSON so valid labels are not silently dropped.

<!-- FIX -->
Fixed learn batch label parsing so numeric and string ids map to candidates, with invalid ids ignored safely.
