---
id: TESTING-1
title: '[CODE-2] UC: Test dual-format parsing for OpenAI response in learn.rs'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-05-05T02:04:47.684473Z'
updated_at: '2026-05-05T02:04:47.684473Z'
tags:
- uc
- test
- openai
version: 1
schema_version: v0.2
due_date: '2026-05-06'
---

<!-- DESCRIPTION -->
Use case: Given a mock OpenAI-format response (choices[0].message.content), When RealBatchLabelDescriber::call_llm parses it, Then it should extract the content correctly.\n\nRef: vtic CODE-2
