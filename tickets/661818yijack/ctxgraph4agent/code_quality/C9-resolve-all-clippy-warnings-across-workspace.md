---
id: C9
title: Resolve all clippy warnings across workspace
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/types.rs,crates/ctxgraph-core/src/graph.rs,crates/ctxgraph-core/src/pattern.rs,crates/ctxgraph-core/src/storage/sqlite.rs,crates/ctxgraph-mcp/src/tools.rs,crates/ctxgraph-cli/src/display.rs
created_at: '2026-04-11T02:03:16.230649Z'
updated_at: '2026-04-11T02:08:31.303758Z'
tags:
- clippy
- code-quality
- refactor
version: 2
---

<!-- DESCRIPTION -->
cargo clippy reports 21 warnings across ctxgraph-core (18), ctxgraph-mcp (1), and ctxgraph-cli (2). Categories: 8 collapsible_if, 2 complex type aliases, 2 unwrap after is_some, 1 dead_code, 1 MutexGuard across await, 1 redundant closure, 1 unwrap_or_default, 1 iter_cloned_collect, 1 filter_map, 1 derivable impl, 1 literal with empty format string. All are safe fixes that improve code quality without changing behavior.
