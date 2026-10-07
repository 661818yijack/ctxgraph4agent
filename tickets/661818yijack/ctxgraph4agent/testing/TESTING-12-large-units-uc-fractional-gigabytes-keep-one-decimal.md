---
id: TESTING-12
title: '[Large units] UC: fractional gigabytes keep one decimal'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:54.896885Z'
updated_at: '2026-10-07T02:06:01.805360Z'
tags:
- daily-20261007
- uc
- stats
- gb
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Use case: given a 1.5 GiB DB, stats should display 1.5 GB.

<!-- FIX -->
Unit test asserts 1.5 GiB formats as 1.5 GB.
