---
id: C7
title: 'MCP e2e tests failing: reqwest blocking client created inside async context'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: critical
status: open
owner: 661818yijack
file: crates/ctxgraph-extract/src/llm_extract.rs
created_at: '2026-04-06T02:04:45.529201Z'
updated_at: '2026-04-06T02:04:45.529201Z'
tags:
- bug
- mcp
- async
- blocking
---

<!-- DESCRIPTION -->
MCP e2e tests panic with 'Cannot drop a runtime in a context where blocking is not allowed'. Root cause: LlmExtractor::from_env() uses reqwest::blocking::Client but is called from Graph::load_extraction_pipeline() inside async fn main(). The blocking client creation (and HTTP calls) must be non-blocking/async.
