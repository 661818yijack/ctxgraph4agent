---
id: TESTING-13
title: '[Large units] UC: terabyte database displays TB'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:55.139524Z'
updated_at: '2026-10-07T02:06:02.067881Z'
tags:
- daily-20261007
- uc
- stats
- tb
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Use case: given a 1 TiB DB, stats should display 1.0 TB instead of 1048576.0 MB.

<!-- FIX -->
Unit test asserts 1 TiB formats as 1.0 TB.
