---
id: T1
title: '[C2] UC: MCP server starts and handles JSON-RPC initialize request successfully'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-06T02:04:48.573004Z'
updated_at: '2026-04-06T02:04:48.573004Z'
tags:
- uc
- mcp
- test
---

<!-- DESCRIPTION -->
Given a spawned ctxgraph-mcp process, when a JSON-RPC initialize request is sent, then the process responds with valid JSON-RPC response (not panic, not EOF).
