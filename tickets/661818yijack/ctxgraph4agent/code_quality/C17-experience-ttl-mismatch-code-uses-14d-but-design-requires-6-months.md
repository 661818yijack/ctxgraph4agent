---
id: C17
title: 'Experience TTL mismatch: code uses 14d but design requires 6 months'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: critical
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-15T02:04:34.054408Z'
updated_at: '2026-04-15T02:10:31.061491Z'
tags:
- bug
- ttl
- experience
- memory-lifecycle
- phase-a
---

<!-- DESCRIPTION -->
CLAUDE.md and design-philosophy docs (updated 2026-04-05) specify that experience TTL is 6 months because raw experiences are the evidence chain behind skills. Compression was removed, so experiences must persist longer. But the actual code still uses 14d in: (1) cleanup_expired hardcoded cutoff, (2) default_ttl_for_memory_type function, (3) migration 003/005 default, (4) types.rs comment. This means experiences are cleaned up after 14 days, destroying the evidence chain that the Learn pipeline (C1a) depends on.
