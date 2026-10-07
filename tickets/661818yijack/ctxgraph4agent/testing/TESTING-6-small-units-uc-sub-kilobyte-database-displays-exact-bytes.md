---
id: TESTING-6
title: '[Small units] UC: sub-kilobyte database displays exact bytes'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:37.793126Z'
updated_at: '2026-10-07T02:05:55.655723Z'
tags:
- daily-20261007
- uc
- stats
- small-units
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Use case: given a DB smaller than 1024 bytes, stats should display the exact byte count with B.

<!-- FIX -->
Unit test asserts sub-kilobyte values format as exact B values.
