---
id: C2
title: "C2: Implicit TTL renewal (recalled and used -> auto-renew)"
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: open
owner: apex-agent
phase: C
priority: P1
effort: M
depends_on:
- A1
- A3
created_at: '2026-04-04T08:55:00.000000Z'
updated_at: '2026-04-05T12:00:00.000000Z'
tags:
- c2
- phase-c
- ttl
- renewal
---

<!-- DESCRIPTION -->
Phase C Story 2 (P1, Medium effort, depends on A1+A3). Implicit TTL renewal: when a memory is recalled via `retrieve_for_context` and its content is used, its TTL is implicitly renewed. Renewal resets `created_at` to now. Gated by `max_renewals` policy (default 5). Uses `renewal_count` (NOT `usage_count`) — renewal_count only increments on actual renewal; usage_count tracks general recall frequency for scoring. Only Facts and Preferences are eligible; Experiences are not.

### Acceptance Criteria:
1. `Storage::renew_memory(id: &str, memory_type: MemoryType) -> Result<bool>` updates `created_at` to now and increments `renewal_count` if renewal is allowed
2. Renewal only applies to `Fact` and `Preference` memory types (not Experience, Pattern, or Decision)
3. Renewal count tracked via `renewal_count` (separate from `usage_count`) — if `renewal_count >= max_renewals`, renewal is denied
4. `MemoryPolicyConfig` has `max_renewals: usize` field (default 5)
5. Renewal returns false (no-op) if memory is already expired (decay_score = 0.0)
6. `retrieve_for_context` automatically calls `renew_memory` for each returned memory (only Facts and Preferences)
7. Renewal only fires for memories that appear in the final returned results (within budget), not all candidates

### Technical Requirements:
- Files to modify: types.rs (add max_renewals to MemoryPolicyConfig), storage/migrations.rs (migration 009: renewal_count INTEGER), storage/sqlite.rs (renew_memory, integrate into retrieve_for_context)
- Config: `max_renewals = 5` in `[policies.<agent>]`

---

## Investigation Notes (2026-04-05)

### Current State: NOTHING auto-renews

The entire implicit TTL renewal mechanism is missing. Here's what exists vs. what's needed:

#### What Exists (Manual Only)
- `Storage::renew_memory_bypass()` (sqlite.rs:1125) — manual renew for reverify CLI only, no `renewal_count` gate, no type filtering
- `touch_entity()` / `touch_edge()` (sqlite.rs:788) — increments `usage_count` + sets `last_recalled_at`, must be called explicitly, not used in retrieval path
- `get_stale_memories()` (sqlite.rs:992) — lists stale memories for reverify CLI
- `cleanup_expired()` (sqlite.rs:1261) — deletes/archive based on TTL+grace, runs every 100 queries

#### What's Missing
| Requirement | Status | Evidence |
|---|---|---|
| `renewal_count` DB column | MISSING | Migration 009 does not exist. DB schema has no `renewal_count` |
| `max_renewals` in policy | MISSING | No `MemoryPolicyConfig` in types.rs. `AgentPolicy` has no `max_renewals` |
| `Storage::renew_memory()` with gate logic | MISSING | Only `renew_memory_bypass` exists (unconditional) |
| `retrieve_for_context` auto-renew | MISSING | Pipeline: candidates → score/rank → enforce_budget → return. No renew step |
| `renewal_count` vs `usage_count` separation | MISSING | No `renewal_count` field anywhere |

#### Key Code Locations
- `retrieve_for_context`: sqlite.rs:2029 — ends at `enforce_budget`, no renewal call after
- `score_candidate`: types.rs — reads `usage_count` for scoring bonus, never writes it
- `touch_entity`/`touch_edge`: sqlite.rs:788 — explicit-only, not called by retrieval
- `maybe_trigger_cleanup`: graph.rs:726 — triggers cleanup every 100 queries, not related to renewal

#### Relevant Files to Modify
1. `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 009 for `renewal_count INTEGER DEFAULT 0`
2. `crates/ctxgraph-core/src/types.rs` — add `max_renewals: usize` to `AgentPolicy` or new `MemoryPolicyConfig`
3. `crates/ctxgraph-core/src/storage/sqlite.rs` — implement `renew_memory()` with gate logic, integrate into `retrieve_for_context`

#### Relationship to Cleanup (A6)
- Cleanup (`cleanup_expired`) runs every 100 queries if last run > 24h ago
- Cleanup uses TTL+grace (Fact=97d, Experience=21d, Preference=37d, Decision=97d) — no 6-month boundary
- Implicit renewal (C2) should run on retrieval, before cleanup ever fires
- These are separate mechanisms: renewal extends life; cleanup deletes expired content
