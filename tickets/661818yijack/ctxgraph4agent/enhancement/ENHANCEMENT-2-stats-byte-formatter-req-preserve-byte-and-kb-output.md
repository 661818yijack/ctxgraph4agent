---
id: ENHANCEMENT-2
title: '[Stats byte formatter] REQ: preserve byte and KB output'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:22.141155Z'
updated_at: '2026-10-07T02:05:45.017089Z'
tags:
- daily-20261007
- req
- stats
- small-units
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Requirement: existing byte and KB outputs must remain stable for small ctxgraph databases.

<!-- FIX -->
Covered zero-byte, sub-KB, and KB cases in unit tests; existing output remains stable.
