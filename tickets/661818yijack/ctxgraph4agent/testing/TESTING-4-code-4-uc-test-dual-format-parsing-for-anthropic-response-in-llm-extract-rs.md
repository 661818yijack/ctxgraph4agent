---
id: TESTING-4
title: '[CODE-4] UC: Test dual-format parsing for Anthropic response in llm_extract.rs'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-05-05T02:05:03.206034Z'
updated_at: '2026-05-05T02:05:03.206034Z'
tags:
- uc
- test
- llm-extract
- anthropic
version: 1
schema_version: v0.2
due_date: '2026-05-06'
---

<!-- DESCRIPTION -->
Use case: Given a mock Anthropic-format response (content[0].text), When LlmExtractor parses it after calling MiniMax endpoint, Then it should extract the JSON content correctly via fallback.\n\nRef: vtic CODE-4
