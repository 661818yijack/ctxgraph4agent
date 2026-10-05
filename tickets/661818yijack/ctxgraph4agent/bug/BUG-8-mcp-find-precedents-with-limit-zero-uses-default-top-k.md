---
id: BUG-8
title: MCP find_precedents with limit zero uses default top K
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:09.134903Z'
updated_at: '2026-10-05T02:08:02.112529Z'
tags:
- apex-daily-20261005
- use-case
- req-search
- uc-precedents-zero
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Requirement: Protect search and precedent limits. Acceptance: find_precedents limit=0 uses default top K.

<!-- FIX -->
find_precedents now uses shared positive limit parsing with default top-K fallback.
