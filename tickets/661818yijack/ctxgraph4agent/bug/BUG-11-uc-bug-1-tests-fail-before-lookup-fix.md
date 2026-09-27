---
id: BUG-11
title: 'UC BUG-1: tests fail before lookup fix'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-27T02:02:48.800495Z'
updated_at: '2026-09-27T02:05:15.661777Z'
tags:
- bug-1
- use-case
- tests
version: 2
schema_version: v0.2
due_date: '2026-09-28'
---

<!-- DESCRIPTION -->
Use case for BUG-1/REQ tests: a regression test should catch current behavior where marked entities are returned.

<!-- FIX -->
Fixed in this branch: direct entity id/name lookups now exclude metadata.marked_for_deletion=true, with regression coverage and workspace tests passing.
