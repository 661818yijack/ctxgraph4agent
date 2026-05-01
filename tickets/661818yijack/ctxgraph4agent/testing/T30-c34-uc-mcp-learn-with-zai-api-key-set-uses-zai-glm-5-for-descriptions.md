---
id: T30
title: '[C34] UC: MCP learn with ZAI_API_KEY set uses ZAI/GLM-5 for descriptions'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-20T02:04:21.656586Z'
updated_at: '2026-04-20T02:04:21.656586Z'
tags:
- uc
- mcp
- zai
- describer
---

<!-- DESCRIPTION -->
Given: ZAI_API_KEY env var is set and valid. When: MCP learn tool is called. Then: BatchLabelDescriber calls ZAI API (https://api.z.ai/api/coding/paas/v4/chat/completions) with glm-5-turbo model, returns real behavioral labels.
