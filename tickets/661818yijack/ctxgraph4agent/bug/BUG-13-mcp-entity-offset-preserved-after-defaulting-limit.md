---
id: BUG-13
title: MCP entity offset preserved after defaulting limit
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:09.835540Z'
updated_at: '2026-10-05T02:08:02.864293Z'
tags:
- apex-daily-20261005
- use-case
- req-list
- uc-offset-preserved
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Requirement: Protect list and export limits. Acceptance: list_entities offset still applies after limit=0 defaults.

<!-- FIX -->
Added test proving list_entities offset is preserved after zero-limit defaulting.
