---
id: USECASE-1
title: 'Use case: FTS search excludes results before --after date'
repo: 661818yijack/ctxgraph4agent
category: use_case
severity: high
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:33.818909Z'
updated_at: '2026-07-14T02:08:18.798587Z'
tags:
- daily-cron
- query-after
- use-case
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of requirement: wire after filter through public search APIs. A query with after=2026-01-01 only returns matching episodes recorded at or after that date.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
