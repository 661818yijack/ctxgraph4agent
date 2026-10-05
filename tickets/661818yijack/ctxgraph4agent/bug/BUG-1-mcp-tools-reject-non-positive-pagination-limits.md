---
id: BUG-1
title: MCP tools reject non-positive pagination limits
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:08.132006Z'
updated_at: '2026-10-05T02:08:01.079814Z'
tags:
- apex-daily-20261005
- parent
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Bug: MCP JSON tools accept limit=0 for search/list/export operations, producing empty or misleading responses instead of a useful default.

<!-- FIX -->
Fixed MCP JSON limit parsing so non-positive and malformed limits use documented defaults instead of returning accidental empty result sets.
