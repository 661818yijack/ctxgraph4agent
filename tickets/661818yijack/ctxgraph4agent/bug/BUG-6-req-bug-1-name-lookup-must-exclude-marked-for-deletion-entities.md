---
id: BUG-6
title: 'REQ BUG-1: name lookup must exclude marked-for-deletion entities'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-27T02:02:48.119749Z'
updated_at: '2026-09-27T02:05:16.679562Z'
tags:
- bug-1
- requirement
- lookup
version: 2
schema_version: v0.2
due_date: '2026-09-28'
---

<!-- DESCRIPTION -->
Requirement for BUG-1: Graph::get_entity_by_name(name) must return None when entity metadata has marked_for_deletion=true.

<!-- FIX -->
Fixed in this branch: direct entity id/name lookups now exclude metadata.marked_for_deletion=true, with regression coverage and workspace tests passing.
