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
completed_at: '2026-04-04T06:48:00+08:00'
commits:
- dbc6750 feat(A4b): scoring and ranking with decay
- 02de10d fix: add NaN guard + missing test coverage
- 71c33e9 fix: remove ineffective NaN test, fix weak assertion
review_loops: 3
tags:
- a4b
- phase-a
- memory-lifecycle
---

<!-- DESCRIPTION -->
Phase A Story 4b (P0, Medium effort, depends on A1+A2+A3+A4a). Implement composite scoring: decay_score * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count)). Patterns get floor 0.5. Expired memories filtered out. Returns Vec<ScoredCandidate> sorted descending.

**Status: COMPLETED** — 3 Codex review loops, APPROVED.
