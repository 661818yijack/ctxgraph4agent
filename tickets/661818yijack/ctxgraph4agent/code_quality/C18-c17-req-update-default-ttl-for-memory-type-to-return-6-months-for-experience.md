---
id: C18
title: '[C17] REQ: Update default_ttl_for_memory_type to return 6 months for Experience'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-15T02:04:35.590766Z'
updated_at: '2026-04-15T02:10:31.235089Z'
tags:
- req
- ttl
- experience
---

<!-- DESCRIPTION -->
The default_ttl_for_memory_type function in sqlite.rs returns 14 days (1209600s) for Experience type. Change to 180 days (15552000s) to match CLAUDE.md spec. Also update the EXPERIENCE_TTL_SECS constant used in decay_score computation.
