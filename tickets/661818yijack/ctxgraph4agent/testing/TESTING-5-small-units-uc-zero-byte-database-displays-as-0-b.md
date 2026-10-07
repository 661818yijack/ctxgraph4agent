---
id: TESTING-5
title: '[Small units] UC: zero-byte database displays as 0 B'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:37.551025Z'
updated_at: '2026-10-07T02:05:45.856447Z'
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
Use case: given an empty or near-new graph DB with zero bytes, stats should display DB size as 0 B.

<!-- FIX -->
Unit test asserts 0 bytes formats as 0 B.
