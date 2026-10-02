---
id: BUG-10
title: 'UC: helper remains deterministic and filesystem-independent'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/mod.rs
created_at: '2026-10-02T02:03:22.861460Z'
updated_at: '2026-10-02T02:06:06.909063Z'
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
Use case under BUG-3: config path derivation should be pure path arithmetic so tests do not require model files or a real database.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.
