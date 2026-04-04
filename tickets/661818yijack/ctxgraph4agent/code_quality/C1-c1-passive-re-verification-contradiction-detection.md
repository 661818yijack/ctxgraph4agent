---
id: C1
title: "C1: Passive re-verification (contradiction detection)"
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: apex-agent
phase: C
priority: P0
effort: L
depends_on:
- A1
- A3
created_at: '2026-04-04T08:55:00.000000Z'
updated_at: '2026-04-04T08:55:00.000000Z'
tags:
- c1
- phase-c
- contradiction
- memory-lifecycle
---

<!-- DESCRIPTION -->
Phase C Story 1 (P0, Low effort, depends on A1+A3). Passive re-verification via contradiction detection. When a new episode is ingested, the system checks existing facts for contradictions. If a new fact conflicts with a stored one (same entity + same relation type but different target), the old edge is invalidated and the new one takes precedence.

### Acceptance Criteria:
1. `Storage::check_contradictions(&self, new_edges: &[Edge]) -> Result<Vec<Contradiction>>` scans for conflicts using entity_id as primary key
2. A contradiction is detected when: same source entity_id (or entity_name as fallback) + same relation type, but different target entity or fact value
3. Contradiction only flagged if existing edge confidence >= `contradiction_threshold` (default 0.2)
4. When contradiction found, the old edge is invalidated (`valid_until = now`) and metadata updated
5. `Contradiction` struct records `{old_edge_id, new_edge_id, entity_id: Option<String>, entity_name: String, relation, old_value, new_value, existing_confidence: f64}`
6. Contradiction invalidation is called automatically during `Graph::add_episode` after extraction
7. Invalidated edges are no longer returned by `get_current_edges_for_entity` but remain in history
8. Entity name normalization: lowercase + trim whitespace before matching

### Technical Requirements:
- Files to modify: types.rs (Contradiction struct), storage/sqlite.rs (check_contradictions, invalidate_contradicted), graph.rs (call in add_episode)
- Config: `contradiction_threshold: f64` (default 0.2) in `[policies.<agent>]`
