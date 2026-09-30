---
id: BUG-9
title: 'UC 2.3: omitted --after remains unfiltered'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:05.982888Z'
updated_at: '2026-09-30T02:05:41.423895Z'
tags:
- after-date-validation
- use-case
- req2
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Use case: User omits --after and should see the normal unfiltered decision list path.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
