---
id: BUG-7
title: 'UC2.1: Long non-ASCII IDs display with ellipsis'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-13T02:01:51.877006Z'
updated_at: '2026-10-13T02:04:30.963783Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-14'
---

<!-- DESCRIPTION -->
Use case: a long non-ASCII memory ID is shortened in the ID column and ends in ellipsis.

<!-- FIX -->
Added long non-ASCII ID regression test; table printing no longer panics.
