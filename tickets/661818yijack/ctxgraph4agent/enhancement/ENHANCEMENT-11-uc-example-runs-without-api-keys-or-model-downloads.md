---
id: ENHANCEMENT-11
title: 'UC: example runs without API keys or model downloads'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: crates/ctxgraph-core/examples/agent_memory.rs
created_at: '2026-08-05T02:02:47.934649Z'
updated_at: '2026-08-05T02:07:22.073704Z'
tags:
- cron
- use-case
- test
version: 2
schema_version: v0.2
due_date: '2026-08-06'
---

<!-- DESCRIPTION -->
Use case: a new evaluator can run the agent memory example with no external services.

<!-- FIX -->
Implemented in branch usecase/agent-memory-sdk-example: added executable agent memory example, use-case tests, README pointer, and fixed traversal edge parameter binding discovered by the use case.
