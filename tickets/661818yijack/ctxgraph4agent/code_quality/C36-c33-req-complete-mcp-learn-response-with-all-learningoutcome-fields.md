---
id: C36
title: '[C33] REQ: Complete MCP learn response with all LearningOutcome fields'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: open
owner: 661818yijack
file: crates/ctxgraph-mcp/src/tools.rs
created_at: '2026-04-20T02:04:00.460702Z'
updated_at: '2026-04-20T02:04:00.460702Z'
tags:
- req
- mcp
- response
- completeness
---

<!-- DESCRIPTION -->
The MCP learn tool response only returns patterns_found, skills_created, skills_updated. It should also return patterns_new and skill_ids from the LearningOutcome struct. These fields are already computed by run_learning_pipeline but discarded in the MCP response.
