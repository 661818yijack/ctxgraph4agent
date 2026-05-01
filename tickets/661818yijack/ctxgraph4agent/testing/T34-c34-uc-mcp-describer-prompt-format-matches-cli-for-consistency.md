---
id: T34
title: '[C34] UC: MCP describer prompt format matches CLI for consistency'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-20T02:04:26.722533Z'
updated_at: '2026-04-20T02:04:26.722533Z'
tags:
- uc
- mcp
- cli
- consistency
---

<!-- DESCRIPTION -->
Given: Both CLI and MCP have BatchLabelDescriber implementations. When: LLM is called. Then: prompt format is identical (numbered pattern list, JSON array output, max 150 chars, no metadata). Both use the same behavioral pattern analyzer prompt.
