---
id: BUG-13
title: 'UC 3.3: unit test accepts missing input'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:06.613940Z'
updated_at: '2026-09-30T02:05:41.957405Z'
tags:
- after-date-validation
- use-case
- req3
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Use case: A test proves no --after input remains accepted as None.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
