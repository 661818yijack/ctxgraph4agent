---
id: BUG-13
title: 'UC BUG-1: no storage schema migration needed'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: null
created_at: '2026-09-27T02:02:49.070962Z'
updated_at: '2026-09-27T02:05:15.941449Z'
tags:
- bug-1
- use-case
- tests
version: 2
schema_version: v0.2
due_date: '2026-09-28'
---

<!-- DESCRIPTION -->
Use case for BUG-1/REQ tests: visibility is enforced in SELECT filters only, without changing database schema.

<!-- FIX -->
Fixed in this branch: direct entity id/name lookups now exclude metadata.marked_for_deletion=true, with regression coverage and workspace tests passing.
