---
id: BUG-6
title: 'REQ: preserve valid --after filtering behavior'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:05.537860Z'
updated_at: '2026-09-30T02:05:41.026112Z'
tags:
- after-date-validation
- requirement
- req2
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Requirement: valid RFC3339 timestamps are still converted to UTC and passed to Graph::list_episodes.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
