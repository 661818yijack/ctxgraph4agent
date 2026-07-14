---
id: REQUIREMENTS-3
title: 'Requirement: update tests and documentation for query date filtering'
repo: 661818yijack/ctxgraph4agent
category: requirements
severity: medium
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:34.985265Z'
updated_at: '2026-07-14T02:08:18.633016Z'
tags:
- daily-cron
- query-after
- requirement
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of parent: CLI query --after flag is ignored. Tests and docs must demonstrate the working --after behavior and guard against regressions.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
