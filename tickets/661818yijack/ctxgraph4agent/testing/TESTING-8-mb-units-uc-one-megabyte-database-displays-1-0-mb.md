---
id: TESTING-8
title: '[MB units] UC: one-megabyte database displays 1.0 MB'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:38.279832Z'
updated_at: '2026-10-07T02:05:56.334505Z'
tags:
- daily-20261007
- uc
- stats
- mb
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Use case: given a 1 MiB DB, stats should display 1.0 MB.

<!-- FIX -->
Unit test asserts 1 MiB formats as 1.0 MB.
