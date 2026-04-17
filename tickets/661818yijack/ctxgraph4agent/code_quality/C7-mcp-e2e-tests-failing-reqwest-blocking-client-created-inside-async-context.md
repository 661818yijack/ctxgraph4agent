---
id: C7
title: 'MCP e2e tests failing: reqwest blocking client created inside async context'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: critical
status: closed
owner: 661818yijack
file: crates/ctxgraph-extract/src/llm_extract.rs
created_at: '2026-04-06T02:04:45.529201Z'
updated_at: '2026-04-15T02:10:57.224274Z'
tags:
- bug
- mcp
- async
- blocking
---

<!-- DESCRIPTION -->
RESOLVED: LlmExtractor was already converted to async reqwest::Client. The MCP server uses tokio async runtime throughout. No reqwest::blocking calls exist in the async path. The only reqwest::blocking usage is in model_manager.rs download() which is only called by ctxgraph models download CLI command, never during MCP operation.
