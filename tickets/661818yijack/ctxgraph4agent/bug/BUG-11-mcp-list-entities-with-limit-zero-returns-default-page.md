---
id: BUG-11
title: MCP list_entities with limit zero returns default page
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:09.553833Z'
updated_at: '2026-10-05T02:08:02.564534Z'
tags:
- apex-daily-20261005
- use-case
- req-list
- uc-entities-zero
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Requirement: Protect list and export limits. Acceptance: list_entities limit=0 returns default page.

<!-- FIX -->
Added test proving list_entities limit=0 returns the default page.
