---
id: BUG-12
title: 'UC: missing config still loads default extraction schema'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/mod.rs
created_at: '2026-10-02T02:03:31.575867Z'
updated_at: '2026-10-02T02:06:07.180233Z'
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
Use case under BUG-4: when no project-root ctxgraph.toml exists, open_graph should continue to call load_extraction_pipeline with defaults.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.
