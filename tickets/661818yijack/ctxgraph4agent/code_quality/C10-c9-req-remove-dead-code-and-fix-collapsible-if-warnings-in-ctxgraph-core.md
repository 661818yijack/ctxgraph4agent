---
id: C10
title: '[C9] REQ: Remove dead code and fix collapsible_if warnings in ctxgraph-core'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/types.rs,crates/ctxgraph-core/src/graph.rs,crates/ctxgraph-core/src/pattern.rs
created_at: '2026-04-11T02:03:19.018693Z'
updated_at: '2026-04-11T02:08:32.549179Z'
tags:
- req
- clippy
- dead-code
- collapsible-if
version: 2
---

<!-- DESCRIPTION -->
Remove unused decay_sigmoid function in types.rs. Collapse 8 nested if statements in graph.rs and pattern.rs into combined let-chains. These are pure style fixes verified by existing tests.
