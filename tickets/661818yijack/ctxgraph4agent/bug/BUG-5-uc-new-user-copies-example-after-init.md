---
id: BUG-5
title: 'UC: New user copies example after init'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: ctxgraph.toml.example
created_at: '2026-10-02T02:03:22.199163Z'
updated_at: '2026-10-02T02:06:06.221691Z'
tags:
- config
- docs
- use-case
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Use case under BUG-2: after ctxgraph init, a user copies ctxgraph.toml.example to ./ctxgraph.toml and expects custom extraction schema to load.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.
