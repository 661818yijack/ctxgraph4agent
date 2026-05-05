---
id: CODE-1
title: 'C7: CLI learn command hardcodes OpenAI-style response parsing for MiniMax'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: open
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/learn.rs
created_at: '2026-05-05T02:04:07.189049Z'
updated_at: '2026-05-05T02:04:07.189049Z'
tags:
- llm
- minimax
- anthropic
- openai
- learn
version: 1
schema_version: v0.2
due_date: '2026-05-06'
---

<!-- DESCRIPTION -->
Root cause: RealBatchLabelDescriber::call_llm() in crates/ctxgraph-cli/src/commands/learn.rs calls MiniMax at https://api.minimax.io/anthropic but parses response with OpenAI path choices[0].message.content. MiniMax returns Anthropic format content[0].text, so ctxgraph learn always fails with 'Invalid LLM response' when using MiniMax keys.\n\nAlso affects: McpBatchLabelDescriber in crates/ctxgraph-mcp/src/tools.rs (has openai_compat flag but still only parses OpenAI format).\n\nAlso affects: LlmExtractor in crates/ctxgraph-extract/src/llm_extract.rs (always uses ChatResponse with choices[] even for MiniMax endpoint).\n\nFix: Dual-format parser — try OpenAI first, fall back to Anthropic format.
