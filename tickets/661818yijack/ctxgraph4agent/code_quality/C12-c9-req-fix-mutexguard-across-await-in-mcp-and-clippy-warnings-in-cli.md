---
id: C12
title: '[C9] REQ: Fix MutexGuard across await in MCP and clippy warnings in CLI'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: crates/ctxgraph-mcp/src/tools.rs,crates/ctxgraph-cli/src/display.rs
created_at: '2026-04-11T02:03:25.279072Z'
updated_at: '2026-04-11T02:08:35.073323Z'
tags:
- req
- clippy
- mcp
- deadlock-potential
version: 2
---

<!-- DESCRIPTION -->
Fix MutexGuard held across await point in ctxgraph-mcp/src/tools.rs:32-34 (warm_cache method). This is a real potential deadlock: std::sync::Mutex held while awaiting tokio::sync::Mutex. Solution: scope the std MutexGuard before the await. Also fix 2 CLI warnings (print_literal with empty format string).
