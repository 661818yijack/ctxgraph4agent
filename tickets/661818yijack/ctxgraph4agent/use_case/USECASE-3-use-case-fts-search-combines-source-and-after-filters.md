---
id: USECASE-3
title: 'Use case: FTS search combines source and after filters'
repo: 661818yijack/ctxgraph4agent
category: use_case
severity: medium
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:34.147044Z'
updated_at: '2026-07-14T02:08:19.137635Z'
tags:
- daily-cron
- query-after
- use-case
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of requirement: wire after filter through public search APIs. Queries with both source and after return only episodes satisfying both filters.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
