---
id: A4b
title: 'A4b: Scoring and ranking with decay'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: completed
owner: apex-agent
created_at: '2026-04-04T06:05:45.080796Z'
updated_at: '2026-04-04T06:05:45.080796Z'
tags:
- a4b
- phase-a
- memory-lifecycle
---

<!-- DESCRIPTION -->
Phase A Story 4b (P0, Medium effort, depends on A1+A2+A3+A4a). Implement composite scoring: decay_score * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count)). Patterns get floor 0.5. Expired memories filtered out. Returns Vec<ScoredCandidate> sorted descending.
