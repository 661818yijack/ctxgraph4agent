---
id: USECASE-2
title: 'Use case: FTS search keeps backward compatibility without after'
repo: 661818yijack/ctxgraph4agent
category: use_case
severity: medium
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:33.982651Z'
updated_at: '2026-07-14T02:08:18.969649Z'
tags:
- daily-cron
- query-after
- use-case
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of requirement: wire after filter through public search APIs. Existing graph.search calls without after continue to return all matching episodes.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
