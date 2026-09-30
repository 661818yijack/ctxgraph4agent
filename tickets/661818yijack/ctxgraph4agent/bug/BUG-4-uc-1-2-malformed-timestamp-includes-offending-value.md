---
id: BUG-4
title: 'UC 1.2: malformed timestamp includes offending value'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:05.223747Z'
updated_at: '2026-09-30T02:05:40.763731Z'
tags:
- after-date-validation
- use-case
- req1
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Use case: User passes a malformed timestamp and the error message should identify the invalid --after value.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
