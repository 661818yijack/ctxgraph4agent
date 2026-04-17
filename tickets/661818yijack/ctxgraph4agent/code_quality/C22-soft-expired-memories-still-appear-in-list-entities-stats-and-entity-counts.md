---
id: C22
title: Soft-expired memories still appear in list_entities, stats, and entity counts
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-17T02:04:54.001857Z'
updated_at: '2026-04-17T02:04:54.001857Z'
tags:
- soft-expire
- forget
- marked_for_deletion
- stats
- list_entities
---

<!-- DESCRIPTION -->
After forget(hard=false), soft-expired memories (marked_for_deletion) still appear in list_entities, get_entity_counts_by_type, stats(), and get_edges_for_entity. PR #6 fixes search/retrieval paths, but enumeration/counting paths remain leaky. Users who soft-expire memories expect them to be invisible across ALL read paths, not just search.
