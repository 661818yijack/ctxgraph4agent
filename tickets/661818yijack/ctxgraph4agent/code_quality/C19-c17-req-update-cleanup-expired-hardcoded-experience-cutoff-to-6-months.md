---
id: C19
title: '[C17] REQ: Update cleanup_expired hardcoded experience cutoff to 6 months'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-15T02:04:41.080594Z'
updated_at: '2026-04-15T02:10:31.405429Z'
tags:
- req
- ttl
- cleanup
- experience
---

<!-- DESCRIPTION -->
The cleanup_expired function in sqlite.rs uses a hardcoded experience_cutoff of 14d (1209600s). Should use 180d (15552000s). This is the runtime cleanup path that deletes old experiences.
