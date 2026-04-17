---
id: C20
title: '[C17] REQ: Update types.rs and CLAUDE.md comments to reflect 6-month experience
  TTL'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/types.rs
created_at: '2026-04-15T02:04:45.658623Z'
updated_at: '2026-04-15T02:10:31.578720Z'
tags:
- req
- docs
- ttl
- experience
---

<!-- DESCRIPTION -->
types.rs line 9 comment says '14d default TTL' for Experience. cleanup_expired comment (line 1668) says 'Experience: 14d'. These must be updated to 6 months / 180d. CLAUDE.md already says 6 months in the lifecycle section but the 'TTL' section at line 37 still says 'experiences: 14d'. Fix all stale references.
