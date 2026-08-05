---
id: ENHANCEMENT-2
title: 'Req: capture sourced tagged decision episodes in agent memory example'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: crates/ctxgraph-core/tests/agent_memory_use_case_test.rs
created_at: '2026-08-05T02:02:46.188213Z'
updated_at: '2026-08-05T02:07:20.131629Z'
tags:
- cron
- use-case
- requirement
version: 2
schema_version: v0.2
due_date: '2026-08-06'
---

<!-- DESCRIPTION -->
Requirement: demonstrate adding a decision episode with source metadata and tags without loading extraction models.

<!-- FIX -->
Implemented in branch usecase/agent-memory-sdk-example: added executable agent memory example, use-case tests, README pointer, and fixed traversal edge parameter binding discovered by the use case.
