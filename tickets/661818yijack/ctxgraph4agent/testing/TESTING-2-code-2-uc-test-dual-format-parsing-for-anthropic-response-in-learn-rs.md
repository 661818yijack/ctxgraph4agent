---
id: TESTING-2
title: '[CODE-2] UC: Test dual-format parsing for Anthropic response in learn.rs'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-05-05T02:04:53.128331Z'
updated_at: '2026-05-05T02:04:53.128331Z'
tags:
- uc
- test
- anthropic
version: 1
schema_version: v0.2
due_date: '2026-05-06'
---

<!-- DESCRIPTION -->
Use case: Given a mock Anthropic-format response (content[0].text), When RealBatchLabelDescriber::call_llm parses it, Then it should extract the content correctly via fallback.\n\nRef: vtic CODE-2
