---
id: T3
title: '[C2] UC: MCP server handles add_episode and search without crashing'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-06T02:04:51.641564Z'
updated_at: '2026-04-06T02:04:51.641564Z'
tags:
- uc
- mcp
- test
---

<!-- DESCRIPTION -->
Given a spawned ctxgraph-mcp process, when add_episode and search tools/call requests are sent, then both return valid JSON responses without panic.
