---
id: C8
title: '[C1] REQ: Change reqwest::blocking::Client to reqwest::Client (async) in LlmExtractor'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-extract/src/llm_extract.rs
created_at: '2026-04-06T02:04:47.028973Z'
updated_at: '2026-04-06T02:04:47.028973Z'
tags:
- req
- async
- llm
---

<!-- DESCRIPTION -->
Replace reqwest::blocking::Client with reqwest::Client (async). Update LlmExtractor methods (extract_entities, extract_relations) to be async. Update pipeline calls to use .await. Must not break sync callers.
