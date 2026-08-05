---
id: ENHANCEMENT-7
title: 'UC: agent links extracted entities back to the source episode'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: crates/ctxgraph-core/tests/agent_memory_use_case_test.rs
created_at: '2026-08-05T02:02:47.152614Z'
updated_at: '2026-08-05T02:07:21.203937Z'
tags:
- cron
- use-case
- test
version: 2
schema_version: v0.2
due_date: '2026-08-06'
---

<!-- DESCRIPTION -->
Use case: manually linking an entity span to an episode supports traceable memory without automatic extraction.

<!-- FIX -->
Implemented in branch usecase/agent-memory-sdk-example: added executable agent memory example, use-case tests, README pointer, and fixed traversal edge parameter binding discovered by the use case.
