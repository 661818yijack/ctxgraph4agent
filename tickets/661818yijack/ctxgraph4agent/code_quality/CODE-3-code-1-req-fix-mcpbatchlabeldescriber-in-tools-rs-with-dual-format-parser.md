---
id: CODE-3
title: '[CODE-1] REQ: Fix McpBatchLabelDescriber in tools.rs with dual-format parser'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-mcp/src/tools.rs
created_at: '2026-05-05T02:04:20.261603Z'
updated_at: '2026-05-05T02:04:20.261603Z'
tags:
- req
- mcp
- dual-format
version: 1
schema_version: v0.2
due_date: '2026-05-06'
---

<!-- DESCRIPTION -->
Requirement: Fix McpBatchLabelDescriber::call_llm() in crates/ctxgraph-mcp/src/tools.rs to parse both OpenAI and Anthropic response formats.\n\nCurrent code has openai_compat flag but still only parses:\njson["choices"][0]["message"]["content"]\n\nWhen openai_compat=false (MiniMax), the response is Anthropic format:\njson["content"][0]["text"]\n\nFix: try OpenAI path first, fall back to Anthropic path regardless of openai_compat flag (defensive).\n\nRef: vtic CODE-1
