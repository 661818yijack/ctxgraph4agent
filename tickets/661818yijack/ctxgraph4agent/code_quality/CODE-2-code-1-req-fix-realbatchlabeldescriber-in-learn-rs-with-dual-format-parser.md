---
id: CODE-2
title: '[CODE-1] REQ: Fix RealBatchLabelDescriber in learn.rs with dual-format parser'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/learn.rs
created_at: '2026-05-05T02:04:13.608840Z'
updated_at: '2026-05-05T02:04:13.608840Z'
tags:
- req
- learn
- dual-format
version: 1
schema_version: v0.2
due_date: '2026-05-06'
---

<!-- DESCRIPTION -->
Requirement: Fix RealBatchLabelDescriber::call_llm() in crates/ctxgraph-cli/src/commands/learn.rs to parse both OpenAI and Anthropic response formats.\n\nCurrent code:\njson["choices"][0]["message"]["content"]\n\nMiniMax returns:\njson["content"][0]["text"]\n\nFix: try OpenAI path first, fall back to Anthropic path.\n\nRef: vtic CODE-1
