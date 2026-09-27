---
id: BUG-1
title: 'BUG: hide soft-deleted entities from direct lookups'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-27T02:02:30.032493Z'
updated_at: '2026-09-27T02:05:15.948089Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-09-28'
---

<!-- DESCRIPTION -->
Parent: direct entity lookups should honor the same marked_for_deletion visibility semantics as list/search/stats so archived or soft-expired memories do not resurface via show-by-id/name.

<!-- FIX -->
Fixed in this branch: direct entity id/name lookups now exclude metadata.marked_for_deletion=true, with regression coverage and workspace tests passing.
