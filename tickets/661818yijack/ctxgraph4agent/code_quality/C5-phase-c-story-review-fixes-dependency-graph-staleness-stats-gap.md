---
id: C5
title: "Phase C story review fixes \u2014 dependency graph staleness + stats gap"
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: open
owner: 661818yijack
file: docs/planning/stories-final.md
created_at: '2026-04-04T06:50:00.413445Z'
updated_at: '2026-04-04T06:50:00.413445Z'
tags:
- phase-c
- review
- dependency-graph
- stats
---

<!-- DESCRIPTION -->
Two issues found during Phase C story review:\n\n1. C3 dependency graph staleness: C3 incorrectly shows depends on A2 (decay_score). C3 queries decay_score at runtime — it doesn't depend on A2 being complete first. Remove A2 from C3 dependencies in both story body (line 640) and dependency graph (line 1055).\n\n2. C4 stats missing contradiction counts: AC8 says stats shows stale/renewed/expired but Known Issues say contradiction counts should also be surfaced. Add contradiction counts to C4 AC8.
