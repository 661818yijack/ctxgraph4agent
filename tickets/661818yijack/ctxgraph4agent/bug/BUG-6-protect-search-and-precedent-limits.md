---
id: BUG-6
title: Protect search and precedent limits
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:08.844816Z'
updated_at: '2026-10-05T02:08:01.815911Z'
tags:
- apex-daily-20261005
- requirement
- req-search
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Parent: MCP tools reject non-positive pagination limits. Search-like MCP tools should return useful defaults when clients pass limit=0 and still honor explicit positive caps.

<!-- FIX -->
Applied shared positive limit parsing to search, find_precedents, and learn paths.
