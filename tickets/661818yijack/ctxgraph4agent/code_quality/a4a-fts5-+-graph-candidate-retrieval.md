---
id: A4a-aa
title: 'A4a: FTS5 + graph candidate retrieval'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: apex-agent
created_at: '2026-04-04T06:05:44.438713Z'
updated_at: '2026-04-04T06:05:44.438713Z'
tags:
- a4a
- phase-a
- memory-lifecycle
---

<!-- DESCRIPTION -->
Phase A Story 4a (P0, Medium effort, depends on A1+A3). Implement candidate retrieval: FTS5 BM25 over entity names/edge labels/episode content + 1-hop graph traversal. Dedup by entity_id/edge_id keeping higher BM25 score. Return Vec<RetrievalCandidate>. Patterns only included if they match query, capped by max_patterns_included.
