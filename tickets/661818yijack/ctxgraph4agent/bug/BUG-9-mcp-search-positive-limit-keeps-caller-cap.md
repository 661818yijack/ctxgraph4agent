---
id: BUG-9
title: MCP search positive limit keeps caller cap
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:09.275023Z'
updated_at: '2026-10-05T02:08:02.264302Z'
tags:
- apex-daily-20261005
- use-case
- req-search
- uc-search-positive
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Requirement: Protect search and precedent limits. Acceptance: search limit=1 returns at most one result.

<!-- FIX -->
Added test proving a positive MCP search limit still caps results.
