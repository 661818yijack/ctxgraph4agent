---
id: USECASE-5
title: 'Use case: CLI query rejects malformed --after'
repo: 661818yijack/ctxgraph4agent
category: use_case
severity: medium
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:34.650252Z'
updated_at: '2026-07-14T02:08:19.480216Z'
tags:
- daily-cron
- query-after
- use-case
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of requirement: parse CLI query --after consistently. A malformed --after value returns a user-visible error and does not run an unfiltered query.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
