---
id: BUG-2
title: 'REQ: parse --after as strict RFC3339'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:04.922813Z'
updated_at: '2026-09-30T02:05:40.500907Z'
tags:
- after-date-validation
- requirement
- req1
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Requirement: malformed --after values return InvalidInput instead of degrading to an unfiltered decision list.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
