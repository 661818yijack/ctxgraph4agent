---
id: BUG-7
title: MCP search with limit zero returns default search results
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:08.990028Z'
updated_at: '2026-10-05T02:08:01.960872Z'
tags:
- apex-daily-20261005
- use-case
- req-search
- uc-search-zero
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Requirement: Protect search and precedent limits. Acceptance: search limit=0 returns the default result page.

<!-- FIX -->
Added test proving MCP search limit=0 returns the same default result page as omitted limit.
