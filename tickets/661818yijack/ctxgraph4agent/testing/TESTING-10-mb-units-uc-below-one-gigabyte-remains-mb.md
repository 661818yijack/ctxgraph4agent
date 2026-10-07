---
id: TESTING-10
title: '[MB units] UC: below one-gigabyte remains MB'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:54.407908Z'
updated_at: '2026-10-07T02:05:56.905146Z'
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
Use case: given a DB just below 1 GiB, stats should still display MB rather than prematurely switching to GB.

<!-- FIX -->
Unit test asserts bytes just below 1 GiB remain formatted as MB.
