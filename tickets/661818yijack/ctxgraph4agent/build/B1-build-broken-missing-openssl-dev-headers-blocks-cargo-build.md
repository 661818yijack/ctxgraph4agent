---
id: B1
title: 'Build broken: missing OpenSSL dev headers blocks cargo build'
repo: 661818yijack/ctxgraph4agent
category: build
severity: critical
status: closed
owner: 661818yijack
file: crates/ctxgraph-cli/Cargo.toml
created_at: '2026-04-05T02:05:42.153862Z'
updated_at: '2026-04-11T02:08:40.350464Z'
tags:
- build
- dependencies
- rustls
version: 2
---

<!-- DESCRIPTION -->
STALE: ctxgraph-cli/Cargo.toml already uses rustls-tls (reqwest = { features = ["blocking", "json", "rustls-tls"] }). Build works without OpenSSL dev headers. No fix needed.
