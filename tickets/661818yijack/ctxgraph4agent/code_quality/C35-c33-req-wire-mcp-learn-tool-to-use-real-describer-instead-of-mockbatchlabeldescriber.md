---
id: C35
title: '[C33] REQ: Wire MCP learn tool to use real describer instead of MockBatchLabelDescriber'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-mcp/src/tools.rs
created_at: '2026-04-20T02:03:59.202246Z'
updated_at: '2026-04-20T02:03:59.202246Z'
tags:
- req
- mcp
- wiring
---

<!-- DESCRIPTION -->
Change the learn method in ToolContext to accept or construct a real BatchLabelDescriber. ToolContext needs access to API keys (from env) to build the describer. The learn tool should silently fall back to mock if no API key is set (silent failure principle).
