---
id: B3
title: '[B2] UC: cargo build --release succeeds without OpenSSL dev headers'
repo: 661818yijack/ctxgraph4agent
category: build
severity: medium
status: open
owner: 661818yijack
file: null
created_at: '2026-04-05T02:05:54.926231Z'
updated_at: '2026-04-05T02:05:54.926231Z'
tags:
- uc
- test
- build
---

<!-- DESCRIPTION -->
Given OpenSSL dev headers are NOT installed on the system,\nWhen the user runs  in the ctxgraph4agent workspace,\nThen the build completes successfully without OpenSSL errors.\n\nThis verifies that switching reqwest to rustls-tls eliminates the native-tls/OpenSSL dependency.
