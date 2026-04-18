---
id: C29
title: '[C25] REQ: Delete marked_for_deletion memories in cleanup_expired sweep'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-18T02:03:43.049353Z'
updated_at: '2026-04-18T02:03:43.049353Z'
tags:
- req
- cleanup
- sweep
- soft-expire
---

<!-- DESCRIPTION -->
cleanup_expired() only checks TTL-based age cutoffs. Soft-expired memories (marked_for_deletion=true) should be deleted regardless of their age. Add a step that queries entities/edges with marked_for_deletion=true, deletes their FK references (episode_entities, edges), then deletes the rows. Count in CleanupResult.
