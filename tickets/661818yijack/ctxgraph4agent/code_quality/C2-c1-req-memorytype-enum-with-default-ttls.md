---
id: C2
title: '[C1] REQ: MemoryType enum with default TTLs'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/types.rs
created_at: '2026-04-04T03:39:00.785408Z'
updated_at: '2026-04-19T02:08:20.801561Z'
tags:
- req
- c1
- enum
---

<!-- DESCRIPTION -->
B2 (Implicit TTL renewal) was removed from the roadmap on 2026-04-05. This ticket is obsolete — auto-renewal on retrieval was deemed redundant: usage_count already rewards frequently-recalled memories in scoring, and active re-verify (B3) handles stale memory detection.
