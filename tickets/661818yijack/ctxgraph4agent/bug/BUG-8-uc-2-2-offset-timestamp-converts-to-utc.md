---
id: BUG-8
title: 'UC 2.2: offset timestamp converts to UTC'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:05.829625Z'
updated_at: '2026-09-30T02:05:41.292434Z'
tags:
- after-date-validation
- use-case
- req2
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Use case: User provides an RFC3339 timestamp with timezone offset and it should normalize to UTC.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
