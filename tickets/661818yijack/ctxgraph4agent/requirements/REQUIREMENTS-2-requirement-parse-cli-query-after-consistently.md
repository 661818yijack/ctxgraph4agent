---
id: REQUIREMENTS-2
title: 'Requirement: parse CLI query --after consistently'
repo: 661818yijack/ctxgraph4agent
category: requirements
severity: high
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:34.309642Z'
updated_at: '2026-07-14T02:08:18.461942Z'
tags:
- daily-cron
- query-after
- requirement
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of parent: CLI query --after flag is ignored. The CLI query command must parse RFC3339 timestamps and reject invalid dates with an error instead of silently ignoring them.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
