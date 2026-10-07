---
id: TESTING-7
title: '[Small units] UC: kilobyte database displays one decimal KB'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:38.036744Z'
updated_at: '2026-10-07T02:05:56.071157Z'
tags:
- daily-20261007
- uc
- stats
- kb
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Use case: given a DB from 1 KiB up to below 1 MiB, stats should display one-decimal KB output.

<!-- FIX -->
Unit test asserts 1024 and 1536 bytes format as one-decimal KB.
