---
id: USECASE-4
title: 'Use case: CLI query passes valid --after into search'
repo: 661818yijack/ctxgraph4agent
category: use_case
severity: high
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:34.480746Z'
updated_at: '2026-07-14T02:08:19.313113Z'
tags:
- daily-cron
- query-after
- use-case
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of requirement: parse CLI query --after consistently. A valid RFC3339 --after value is converted to UTC and passed into the search path.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
