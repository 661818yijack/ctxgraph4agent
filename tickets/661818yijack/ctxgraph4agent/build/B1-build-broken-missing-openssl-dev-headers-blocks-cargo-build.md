---
id: B1
title: 'Build broken: missing OpenSSL dev headers blocks cargo build'
repo: 661818yijack/ctxgraph4agent
category: build
severity: critical
status: open
owner: 661818yijack
file: crates/ctxgraph-cli/Cargo.toml
created_at: '2026-04-05T02:05:42.153862Z'
updated_at: '2026-04-05T02:05:42.153862Z'
tags:
- build
- dependencies
- rustls
---

<!-- DESCRIPTION -->
ctxgraph-cli/Cargo.toml uses reqwest with default features (native-tls) which requires OpenSSL dev headers. On systems without libssl-dev installed, cargo build fails with:\n  error: failed to run custom build command for openssl-sys\n  warning: Could not find directory of OpenSSL installation\n\nFix: change reqwest feature from default to explicit rustls-tls, same as ctxgraph-extract already does. This eliminates the OpenSSL dependency entirely.\n\nAffected file: crates/ctxgraph-cli/Cargo.toml line 24\nAffected crate: ctxgraph-cli\nNo tests currently cover the build failure scenario.
