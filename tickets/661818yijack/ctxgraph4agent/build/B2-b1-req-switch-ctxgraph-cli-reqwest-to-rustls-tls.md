---
id: B2
title: '[B1] REQ: Switch ctxgraph-cli reqwest to rustls-tls'
repo: 661818yijack/ctxgraph4agent
category: build
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-cli/Cargo.toml
created_at: '2026-04-05T02:05:47.357023Z'
updated_at: '2026-04-05T02:05:47.357023Z'
tags:
- req
- build
- rustls
---

<!-- DESCRIPTION -->
Change reqwest dependency in ctxgraph-cli/Cargo.toml from default features to explicit rustls-tls. ctxgraph-extract already uses rustls-tls. This eliminates the OpenSSL sys dependency entirely.
