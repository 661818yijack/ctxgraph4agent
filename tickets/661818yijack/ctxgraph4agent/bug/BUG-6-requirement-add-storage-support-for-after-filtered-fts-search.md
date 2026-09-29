---
id: BUG-6
title: 'Requirement: add storage support for after-filtered FTS search'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-29T02:02:15.756730Z'
updated_at: '2026-09-29T02:06:10.957195Z'
tags:
- parent:query-after-filter
version: 2
schema_version: v0.2
due_date: '2026-09-30'
---

<!-- DESCRIPTION -->
The FTS episode search path accepts an optional after timestamp and applies it in SQL before ranking/limit.

<!-- FIX -->
Implemented after-filtered FTS search for ctxgraph query, covered date/source/limit behavior with regression tests, and verified cargo test --workspace --no-default-features passes.
