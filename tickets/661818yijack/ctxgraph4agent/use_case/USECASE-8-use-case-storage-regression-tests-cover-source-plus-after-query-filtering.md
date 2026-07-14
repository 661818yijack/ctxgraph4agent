---
id: USECASE-8
title: 'Use case: storage regression tests cover source-plus-after query filtering'
repo: 661818yijack/ctxgraph4agent
category: use_case
severity: medium
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:33.516426Z'
updated_at: '2026-07-14T02:08:19.997486Z'
tags:
- daily-cron
- query-after
- use-case
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of requirement: update tests and documentation for query date filtering. Tests fail if combined filters leak wrong source or old matches.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
