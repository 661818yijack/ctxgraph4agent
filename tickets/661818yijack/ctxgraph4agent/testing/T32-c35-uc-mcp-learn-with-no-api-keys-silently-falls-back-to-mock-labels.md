---
id: T32
title: '[C35] UC: MCP learn with no API keys silently falls back to mock labels'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-20T02:04:24.187969Z'
updated_at: '2026-04-20T02:04:24.187969Z'
tags:
- uc
- mcp
- silent-fallback
---

<!-- DESCRIPTION -->
Given: No API keys are set (ZAI_API_KEY, MINIMAX_API_KEY, CTXGRAPH_LLM_KEY all missing). When: MCP learn tool is called. Then: silently uses MockBatchLabelDescriber, no error printed, no warning, skills still created with mock labels.
