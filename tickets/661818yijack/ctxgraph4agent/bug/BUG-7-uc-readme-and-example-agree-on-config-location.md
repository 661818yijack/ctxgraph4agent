---
id: BUG-7
title: 'UC: README and example agree on config location'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: ctxgraph.toml.example
created_at: '2026-10-02T02:03:22.460115Z'
updated_at: '2026-10-02T02:06:06.494762Z'
tags:
- config
- docs
- use-case
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Use case under BUG-2: the example should align with README quick-start wording that presents ctxgraph.toml as a project-root file.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.
