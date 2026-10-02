---
id: BUG-13
title: 'UC: bad extraction setup remains non-fatal'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/mod.rs
created_at: '2026-10-02T02:03:31.710137Z'
updated_at: '2026-10-02T02:06:07.322551Z'
tags:
- config
- cli
- regression
- use-case
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Use case under BUG-4: extraction pipeline load errors should still warn and continue opening the graph rather than blocking graph access.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.
