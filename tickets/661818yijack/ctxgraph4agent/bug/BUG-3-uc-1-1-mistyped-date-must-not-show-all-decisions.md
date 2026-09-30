---
id: BUG-3
title: 'UC 1.1: mistyped date must not show all decisions'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:05.078131Z'
updated_at: '2026-09-30T02:05:40.632594Z'
tags:
- after-date-validation
- use-case
- req1
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Use case: User mistypes a date such as 2026-09-30 and should see a clear error rather than all decisions.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
