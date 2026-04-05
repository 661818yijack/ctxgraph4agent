---
id: C6
title: 'A6: TTL enforcement and cleanup'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: apex-agent
file: tickets/661818yijack/ctxgraph4agent/code_quality/a6-ttl-enforcement-and-cleanup.md
created_at: '2026-04-05T01:44:41.890676Z'
updated_at: '2026-04-05T01:44:41.890676Z'
tags:
- a6
- phase-a
- memory-lifecycle
---

<!-- DESCRIPTION -->
Phase A Story 6 (P0, Large effort, depends on A1+A2+A3). Implement cleanup_expired: delete Facts/Experiences with decay_score=0 past grace_period, archive Preferences/Decisions. Patterns never cleaned. Lazy trigger (every N queries). system_metadata table for last_cleanup_at. cleanup_in_progress flag. CleanupResult struct.
