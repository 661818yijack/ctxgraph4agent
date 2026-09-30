---
id: BUG-10
title: 'REQ: cover decision date parsing with tests'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph-cli/src/commands/decisions.rs
created_at: '2026-09-30T02:02:06.133956Z'
updated_at: '2026-09-30T02:05:41.557638Z'
tags:
- after-date-validation
- requirement
- req3
version: 2
schema_version: v0.2
due_date: '2026-10-01'
---

<!-- DESCRIPTION -->
Requirement: tests cover invalid, absent, and valid --after inputs before implementation changes.

<!-- FIX -->
Implemented strict RFC3339 validation for ctxgraph decisions list --after; malformed timestamps now fail closed with InvalidInput, valid/missing filters are covered by unit tests and manual CLI verification.
