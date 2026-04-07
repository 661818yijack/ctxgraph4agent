---
id: T2
title: '[C2] UC: MCP server handles tools/list request without crashing'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-06T02:04:50.098214Z'
updated_at: '2026-04-06T02:04:50.098214Z'
tags:
- uc
- mcp
- test
---

<!-- DESCRIPTION -->
Given a spawned ctxgraph-mcp process that has completed initialize handshake, when tools/list is sent, then the process returns a valid tools list response.
