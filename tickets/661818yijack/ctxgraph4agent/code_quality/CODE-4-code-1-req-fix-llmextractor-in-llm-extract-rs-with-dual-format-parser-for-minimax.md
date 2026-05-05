---
id: CODE-4
title: '[CODE-1] REQ: Fix LlmExtractor in llm_extract.rs with dual-format parser for
  MiniMax'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-extract/src/llm_extract.rs
created_at: '2026-05-05T02:04:41.717530Z'
updated_at: '2026-05-05T02:04:41.717530Z'
tags:
- req
- llm-extract
- dual-format
version: 1
schema_version: v0.2
due_date: '2026-05-06'
---

<!-- DESCRIPTION -->
Requirement: Fix LlmExtractor in crates/ctxgraph-extract/src/llm_extract.rs to parse both OpenAI and Anthropic response formats.\n\nCurrent code uses ChatResponse struct with choices[] which only works for OpenAI format. When calling MiniMax endpoint (https://api.minimax.io/anthropic/chat/completions), the response is Anthropic format with content[] array.\n\nFix: Add dual-format parsing — try ChatResponse (OpenAI) first, fall back to raw JSON with content[0].text (Anthropic).\n\nRef: vtic CODE-1
