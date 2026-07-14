---
id: BUG-1
title: CLI query --after flag is ignored
repo: 661818yijack/ctxgraph4agent
category: bug
severity: high
status: done
owner: Apex
file: null
created_at: '2026-07-14T02:02:33.485329Z'
updated_at: '2026-07-14T02:08:18.120524Z'
tags:
- daily-cron
- query-after
- parent
version: 2
schema_version: v0.2
due_date: '2026-07-15'
---

<!-- DESCRIPTION -->
Parent: fix ctxgraph query so the advertised --after filter limits FTS5 episode search results by recorded_at. Daily CTO bug-fix mission 2026-07-14.

<!-- FIX -->
Implemented and verified by Apex: query --after now filters FTS search by recorded_at, combines with source filtering, rejects malformed CLI timestamps, and is covered by tests. cargo test --workspace and cargo clippy --workspace -- -D warnings passed.
