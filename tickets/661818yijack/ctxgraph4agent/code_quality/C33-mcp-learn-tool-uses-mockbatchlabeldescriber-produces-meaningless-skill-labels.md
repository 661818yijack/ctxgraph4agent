---
id: C33
title: "MCP learn tool uses MockBatchLabelDescriber \u2014 produces meaningless skill\
  \ labels"
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-mcp/src/tools.rs
created_at: '2026-04-20T02:03:43.130194Z'
updated_at: '2026-04-20T02:03:43.130194Z'
tags:
- mcp
- learn
- quality
- enhancement
---

<!-- DESCRIPTION -->
The MCP learn tool (tools.rs:learn) uses MockBatchLabelDescriber which generates placeholder labels like 'Pattern 1'. The CLI has RealBatchLabelDescriber with actual LLM calls, but MCP server doesn't have access to it. Agents using the MCP interface get useless skills. Also the MCP response is missing fields (patterns_new, skill_ids) that the LearningOutcome struct contains.
