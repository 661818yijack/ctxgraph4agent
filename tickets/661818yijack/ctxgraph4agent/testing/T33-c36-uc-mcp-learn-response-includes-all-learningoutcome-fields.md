---
id: T33
title: '[C36] UC: MCP learn response includes all LearningOutcome fields'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-20T02:04:25.462138Z'
updated_at: '2026-04-20T02:04:25.462138Z'
tags:
- uc
- mcp
- response
- completeness
---

<!-- DESCRIPTION -->
Given: Learning pipeline produces results. When: MCP learn tool returns. Then: response JSON contains patterns_found, patterns_new, skills_created, skills_updated, and skill_ids fields matching LearningOutcome struct.
