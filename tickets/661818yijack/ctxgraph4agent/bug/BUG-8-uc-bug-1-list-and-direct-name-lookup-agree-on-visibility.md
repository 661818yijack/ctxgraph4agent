---
id: BUG-8
title: 'UC BUG-1: list and direct name lookup agree on visibility'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-27T02:02:48.386817Z'
updated_at: '2026-09-27T02:05:15.217835Z'
tags:
- bug-1
- use-case
- lookup
version: 2
schema_version: v0.2
due_date: '2026-09-28'
---

<!-- DESCRIPTION -->
Use case for BUG-1/REQ name lookup: an entity hidden from list_entities should also be hidden from get_entity_by_name.

<!-- FIX -->
Fixed in this branch: direct entity id/name lookups now exclude metadata.marked_for_deletion=true, with regression coverage and workspace tests passing.
