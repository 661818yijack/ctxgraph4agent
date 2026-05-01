---
id: C34
title: '[C33] REQ: Add real LLM-based BatchLabelDescriber to MCP crate'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-mcp/src/tools.rs
created_at: '2026-04-20T02:03:57.942071Z'
updated_at: '2026-04-20T02:03:57.942071Z'
tags:
- req
- mcp
- llm
- describer
---

<!-- DESCRIPTION -->
Create an MCP-native BatchLabelDescriber that makes LLM calls using env vars (ZAI_API_KEY, MINIMAX_API_KEY, CTXGRAPH_LLM_KEY). Follow the same provider priority as llm_extract.rs. Reuse the prompt format from CLI's RealBatchLabelDescriber. Must be async and implement BatchLabelDescriber trait.
