---
id: ENHANCEMENT-3
title: '[Stats byte formatter] REQ: preserve MB output'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:22.383885Z'
updated_at: '2026-10-07T02:05:45.270034Z'
tags:
- daily-20261007
- req
- stats
- mb
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Requirement: existing MB output must remain stable for medium ctxgraph databases.

<!-- FIX -->
Covered 1.0 MB, fractional MB, and just-below-GB cases in unit tests; MB behavior remains stable.
