---
id: BUG-11
title: 'UC 3.1: unit test rejects date-only input'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:06.288433Z'
updated_at: '2026-09-30T02:05:41.691781Z'
tags:
- after-date-validation
- use-case
- req3
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Use case: A test proves date-only input is rejected instead of silently ignored.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
