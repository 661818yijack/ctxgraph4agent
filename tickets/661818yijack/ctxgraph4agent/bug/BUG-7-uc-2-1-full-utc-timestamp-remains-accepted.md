---
id: BUG-7
title: 'UC 2.1: full UTC timestamp remains accepted'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:05.683817Z'
updated_at: '2026-09-30T02:05:41.158520Z'
tags:
- after-date-validation
- use-case
- req2
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Use case: User provides a full RFC3339 timestamp with Z and should get normal filtered listing behavior.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
