---
id: C15
title: '[C13] REQ: Remove dead decay_sigmoid function from types.rs'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/types.rs
created_at: '2026-04-13T02:04:55.172311Z'
updated_at: '2026-04-13T02:09:37.405691Z'
tags:
- req
- dead-code
- clippy
version: 3
---

<!-- DESCRIPTION -->
decay_sigmoid() in crates/ctxgraph-core/src/types.rs is dead code. It was used by Decision decay before PR #2 changed Decision to exponential (same as Fact). No callers remain. This causes `cargo clippy -- -D warnings` to fail. Remove the function and its doc comment.
