---
id: C11
title: '[C9] REQ: Fix clippy warnings in sqlite.rs and graph.rs (unwrap, redundant
  closure, etc.)'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs,crates/ctxgraph-core/src/graph.rs
created_at: '2026-04-11T02:03:22.010782Z'
updated_at: '2026-04-11T02:08:33.808581Z'
tags:
- req
- clippy
- refactor
version: 2
---

<!-- DESCRIPTION -->
Fix: unwrap_or_default in sqlite.rs:513, iter_cloned_collect in sqlite.rs:948, redundant closure in graph.rs:997, filter_map simplification, unwrap after is_some checks (2 instances), derivable impl, complex type aliases (2 instances). All pure refactor, existing tests verify.
