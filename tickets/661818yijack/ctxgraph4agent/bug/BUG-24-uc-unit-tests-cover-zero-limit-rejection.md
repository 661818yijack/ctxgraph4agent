---
id: BUG-24
title: 'UC: unit tests cover zero limit rejection'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-12T02:02:12.140564Z'
updated_at: '2026-10-12T02:04:59.762471Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-13'
---

<!-- DESCRIPTION -->
Use case under BUG-14: tests fail if zero limits are accepted.

<!-- FIX -->
Regression test asserts zero limit returns an InvalidInput error.
