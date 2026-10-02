---
id: BUG-3
title: 'REQ: Test config path derivation from graph.db'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/mod.rs
created_at: '2026-10-02T02:03:07.750669Z'
updated_at: '2026-10-02T02:06:05.949800Z'
tags:
- config
- cli
- test
- requirement
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Requirement for parent config-path alignment: add a focused test proving .ctxgraph/graph.db resolves to <project>/ctxgraph.toml.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.
