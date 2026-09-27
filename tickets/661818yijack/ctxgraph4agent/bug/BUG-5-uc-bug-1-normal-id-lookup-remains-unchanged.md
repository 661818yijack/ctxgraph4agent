---
id: BUG-5
title: 'UC BUG-1: normal id lookup remains unchanged'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: null
created_at: '2026-09-27T02:02:47.986666Z'
updated_at: '2026-09-27T02:05:16.521453Z'
tags:
- bug-1
- use-case
- lookup
version: 2
schema_version: v0.2
due_date: '2026-09-28'
---

<!-- DESCRIPTION -->
Use case for BUG-1/REQ ID lookup: unmarked entities still resolve by id.

<!-- FIX -->
Fixed in this branch: direct entity id/name lookups now exclude metadata.marked_for_deletion=true, with regression coverage and workspace tests passing.
