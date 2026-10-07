---
id: ENHANCEMENT-1
title: Stats CLI byte formatter uses scalable units
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:21.898734Z'
updated_at: '2026-10-07T02:05:44.639352Z'
tags:
- daily-20261007
- parent
- stats
- byte-format
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Parent: improve ctxgraph stats DB size display so large databases are reported with readable units beyond MB while preserving existing small-size output.

<!-- FIX -->
Implemented scalable stats DB-size formatting with preserved B/KB/MB behavior plus GB/TB support and tests.
