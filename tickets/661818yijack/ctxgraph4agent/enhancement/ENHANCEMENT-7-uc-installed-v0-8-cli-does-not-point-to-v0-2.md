---
id: ENHANCEMENT-7
title: 'UC: Installed v0.8 CLI does not point to v0.2'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-03T02:03:25.433133Z'
updated_at: '2026-10-03T02:05:42.666669Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-04'
---

<!-- DESCRIPTION -->
Use case: a user running current binaries should not see old version references in command output.

<!-- FIX -->
Added regression coverage ensuring v0.2 is absent from entities guidance source.
