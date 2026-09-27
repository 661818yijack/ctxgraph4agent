---
id: BUG-10
title: 'REQ BUG-1: regression tests cover soft-delete visibility'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-27T02:02:48.661662Z'
updated_at: '2026-09-27T02:05:15.523665Z'
tags:
- bug-1
- requirement
- tests
version: 2
schema_version: v0.2
due_date: '2026-09-28'
---

<!-- DESCRIPTION -->
Requirement for BUG-1: add focused tests proving direct lookup visibility matches soft-delete policy.

<!-- FIX -->
Fixed in this branch: direct entity id/name lookups now exclude metadata.marked_for_deletion=true, with regression coverage and workspace tests passing.
