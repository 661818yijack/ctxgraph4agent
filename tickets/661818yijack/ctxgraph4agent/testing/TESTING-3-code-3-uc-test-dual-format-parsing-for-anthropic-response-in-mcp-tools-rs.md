---
id: TESTING-3
title: '[CODE-3] UC: Test dual-format parsing for Anthropic response in MCP tools.rs'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-05-05T02:04:58.527307Z'
updated_at: '2026-05-05T02:04:58.527307Z'
tags:
- uc
- test
- mcp
- anthropic
version: 1
schema_version: v0.2
due_date: '2026-05-06'
---

<!-- DESCRIPTION -->
Use case: Given a mock Anthropic-format response (content[0].text), When McpBatchLabelDescriber::call_llm parses it with openai_compat=false, Then it should extract the content correctly via fallback.\n\nRef: vtic CODE-3
