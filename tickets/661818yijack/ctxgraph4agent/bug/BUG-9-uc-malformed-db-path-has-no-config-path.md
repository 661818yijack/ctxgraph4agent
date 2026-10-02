---
id: BUG-9
title: 'UC: malformed db path has no config path'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/mod.rs
created_at: '2026-10-02T02:03:22.728755Z'
updated_at: '2026-10-02T02:06:06.770229Z'
tags:
- config
- cli
- test
- use-case
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Use case under BUG-3: a db path without the expected .ctxgraph parent should not panic during config path derivation.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.
