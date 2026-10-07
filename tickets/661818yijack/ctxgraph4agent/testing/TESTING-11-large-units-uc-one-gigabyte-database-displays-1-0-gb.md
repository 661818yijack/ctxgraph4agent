---
id: TESTING-11
title: '[Large units] UC: one-gigabyte database displays 1.0 GB'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:54.654401Z'
updated_at: '2026-10-07T02:06:01.562270Z'
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
Use case: given a 1 GiB DB, stats should display 1.0 GB instead of 1024.0 MB.

<!-- FIX -->
Unit test asserts 1 GiB formats as 1.0 GB.
