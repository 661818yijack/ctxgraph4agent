---
id: T31
title: '[C34] UC: MCP learn with MINIMAX_API_KEY uses MiniMax for descriptions'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-20T02:04:22.920723Z'
updated_at: '2026-04-20T02:04:22.920723Z'
tags:
- uc
- mcp
- minimax
- describer
---

<!-- DESCRIPTION -->
Given: MINIMAX_API_KEY is set (no ZAI_API_KEY). When: MCP learn tool is called. Then: BatchLabelDescriber calls MiniMax API (https://api.minimax.io/anthropic/v1/messages) with x-api-key header.
