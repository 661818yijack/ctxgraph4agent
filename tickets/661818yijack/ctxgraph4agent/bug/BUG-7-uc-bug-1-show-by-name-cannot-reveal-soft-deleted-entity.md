---
id: BUG-7
title: 'UC BUG-1: show by name cannot reveal soft-deleted entity'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-27T02:02:48.253538Z'
updated_at: '2026-09-27T02:05:15.071159Z'
tags:
- bug-1
- use-case
- lookup
version: 2
schema_version: v0.2
due_date: '2026-09-28'
---

<!-- DESCRIPTION -->
Use case for BUG-1/REQ name lookup: after cleanup marks an entity, direct name lookup should behave as not found.

<!-- FIX -->
Fixed in this branch: direct entity id/name lookups now exclude metadata.marked_for_deletion=true, with regression coverage and workspace tests passing.
