---
id: C33
title: 'C37: Learn pipeline broken - get_pattern_candidates returned empty (compression
  removed)'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: critical
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-21T02:14:44.552390Z'
updated_at: '2026-04-21T02:14:52.895119Z'
tags:
- critical
- bug
- learn
- pipeline
- fixed
---

<!-- DESCRIPTION -->
CRITICAL BUG fixed in PR #11. Storage::get_pattern_candidates() referenced removed compression system. Rewrote to use raw Experience episodes. Removed dead get_compression_groups() and CompressionGroupData.
