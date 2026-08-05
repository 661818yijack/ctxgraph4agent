---
id: ENHANCEMENT-5
title: 'UC: full-text search finds the recorded decision'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: crates/ctxgraph-core/tests/agent_memory_use_case_test.rs
created_at: '2026-08-05T02:02:46.737972Z'
updated_at: '2026-08-05T02:07:20.783623Z'
tags:
- cron
- use-case
- test
version: 2
schema_version: v0.2
due_date: '2026-08-06'
---

<!-- DESCRIPTION -->
Use case: an agent asks why a stored choice was made and receives the matching episode through local FTS.

<!-- FIX -->
Implemented in branch usecase/agent-memory-sdk-example: added executable agent memory example, use-case tests, README pointer, and fixed traversal edge parameter binding discovered by the use case.
