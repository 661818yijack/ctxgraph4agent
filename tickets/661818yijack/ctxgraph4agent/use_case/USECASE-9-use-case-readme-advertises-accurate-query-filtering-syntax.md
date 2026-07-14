---
id: USECASE-9
title: 'Use case: README advertises accurate query filtering syntax'
repo: 661818yijack/ctxgraph4agent
category: use_case
severity: low
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:33.689894Z'
updated_at: '2026-07-14T02:08:20.164726Z'
tags:
- daily-cron
- query-after
- use-case
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of requirement: update tests and documentation for query date filtering. User-facing docs show --after on query with RFC3339 format.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
