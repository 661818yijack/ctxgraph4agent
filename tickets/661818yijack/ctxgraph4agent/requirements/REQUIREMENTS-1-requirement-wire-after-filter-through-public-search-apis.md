---
id: REQUIREMENTS-1
title: 'Requirement: wire after filter through public search APIs'
repo: 661818yijack/ctxgraph4agent
category: requirements
severity: high
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:33.653687Z'
updated_at: '2026-07-14T02:08:18.287522Z'
tags:
- daily-cron
- query-after
- requirement
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Child of parent: CLI query --after flag is ignored. Graph::search and Storage::search_episodes must accept an optional DateTime<Utc> after filter while preserving source filtering.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
