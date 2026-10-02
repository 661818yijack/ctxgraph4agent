---
id: BUG-8
title: 'UC: graph.db inside .ctxgraph resolves to root config'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/mod.rs
created_at: '2026-10-02T02:03:22.591571Z'
updated_at: '2026-10-02T02:06:06.632553Z'
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
Use case under BUG-3: /work/proj/.ctxgraph/graph.db must resolve to /work/proj/ctxgraph.toml.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.
