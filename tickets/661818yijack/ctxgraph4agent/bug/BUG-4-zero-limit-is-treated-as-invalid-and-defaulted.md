---
id: BUG-4
title: Zero limit is treated as invalid and defaulted
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:08.561754Z'
updated_at: '2026-10-05T02:08:01.520399Z'
tags:
- apex-daily-20261005
- use-case
- req-input
- uc-zero-limit
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Requirement: Normalize MCP positive limits. Acceptance: limit=0 falls back to the tool default.

<!-- FIX -->
Changed limit=0 behavior to fall back to the tool default.
