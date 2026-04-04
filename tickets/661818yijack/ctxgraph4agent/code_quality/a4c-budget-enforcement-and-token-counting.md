---
id: A4c
title: 'A4c: Budget enforcement and token counting'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: apex-agent
created_at: '2026-04-04T06:05:45.717932Z'
updated_at: '2026-04-04T06:05:45.717932Z'
tags:
- a4c
- phase-a
- memory-lifecycle
---

<!-- DESCRIPTION -->
Phase A Story 4c (P0, Small effort, depends on A4a+A4b). Implement greedy budget enforcement: add highest-scored candidates first until budget exhausted. Token estimate: text.len()/4. Single memory exceeding budget is skipped. Returns (Vec<RankedMemory>, tokens_spent). Orchestrates A4a+A4b+A4c via retrieve_for_context.
