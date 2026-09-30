---
id: BUG-1
title: 'BUG: reject invalid --after timestamps for decision listing'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:04.775910Z'
updated_at: '2026-09-30T02:05:40.368994Z'
tags:
- after-date-validation
- parent
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Parent: CLI decision listing must not silently ignore malformed --after filters because users rely on time filters for recall.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
