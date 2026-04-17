---
id: C16
title: '[C13] REQ: Add contradiction count to GraphStats'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/types.rs
created_at: '2026-04-13T02:04:56.527284Z'
updated_at: '2026-04-13T02:05:17.490064Z'
tags:
- req
- stats
- contradictions
version: 2
---

<!-- DESCRIPTION -->
DEFRERED: Contradictions are not persisted to a table — they are computed on-the-fly by check_contradictions(). Adding a count would require either persisting contradictions or scanning all edges at stats time, both over-engineering for current needs. Revisit if contradiction tracking is needed for re-verify workflow (B1).
