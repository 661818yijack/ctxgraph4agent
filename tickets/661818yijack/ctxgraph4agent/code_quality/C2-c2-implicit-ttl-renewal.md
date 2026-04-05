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
- A6
created_at: '2026-04-04T08:55:00.000000Z'
updated_at: '2026-04-05T14:00:00.000000Z'
tags:
- c2
- phase-c
- ttl
- renewal
---

<!-- DESCRIPTION -->
Phase C Story 2 (P1, Medium effort, depends on A1+A6). Implicit TTL renewal: when a memory is recalled via `retrieve_for_context` and its content is used, its TTL is implicitly renewed. Renewal resets `created_at` to now. Gated by `max_renewals` policy (default 5). Only Facts and Preferences are eligible; Experiences are not.

### Acceptance Criteria:
1. `renewal_count` column exists in `entities` and `edges` tables (migration 009)
2. `renew_memory(id, memory_type)` checks `renewal_count < max_renewals` before renewing; increments `renewal_count` on success
3. Renewal only applies to `Fact` and `Preference` memory types (not Experience, Pattern, or Decision)
4. Renewal returns false (no-op) if memory is already expired (decay_score = 0.0)
5. `retrieve_for_context` automatically calls `touch_entity`/`touch_edge` for each returned memory (update usage_count + last_recalled_at)
6. Budget-enforced results (final returned set) trigger `renew_memory` for Facts and Preferences only
7. `max_renewals` is configurable per-agent in `AgentPolicy` (default: 5)

### Technical Requirements:
- Files to modify:
  - `storage/migrations.rs` — add migration 009: `renewal_count INTEGER DEFAULT 0` on entities and edges
  - `types.rs` — add `max_renewals: usize` to `AgentPolicy`
  - `storage/sqlite.rs` — implement gated `renew_memory()`, wire `touch_*` into `retrieve_candidates`, call `renew_memory` on budget-enforced results
- `max_renewals = 5` default in `[policies.<agent>]`

---

## Investigation Notes (2026-04-05, updated)

### Current State: Partial Implementation

#### What Exists
| Function | Location | Purpose |
|----------|----------|---------|
| `touch_entity(id)` | sqlite.rs:1076 | Increment `usage_count`, set `last_recalled_at` |
| `touch_edge(id)` | sqlite.rs:1090 | Increment `usage_count`, set `last_recalled_at` |
| `renew_memory_bypass(id, memory_type)` | sqlite.rs:1272 | Reset `created_at` + `recorded_at`, set new TTL (unconditional, no renewal_count gate) |
| `Graph::renew_memory(id, memory_type)` | graph.rs:697 | Passthrough to `renew_memory_bypass` |
| `maybe_trigger_cleanup` | graph.rs | Lazy cleanup trigger every 100 queries |

#### What's Missing (the actual gap)
| Requirement | Status |
|---|---|
| `renewal_count` DB column | MISSING — no migration 009 |
| `max_renewals` in `AgentPolicy` | MISSING — `AgentPolicy` has no max_renewals |
| Gated `renew_memory()` with `renewal_count < max_renewals` | MISSING — current `renew_memory_bypass` is unconditional |
| `touch_*` wired into `retrieve_candidates` | MISSING — `touch_*` exist but never called during retrieval |
| Budget-enforced results trigger `renew_memory` | MISSING — renewal happens nowhere in retrieval path |
| Type filtering (Fact+Preference only) | MISSING — current renew bypasses apply to all types |

#### Retrieval Pipeline (what actually runs)
```
retrieve_candidates (sqlite.rs:2037)
  → FTS5 entity search
  → FTS5 edge search
  → FTS5 episode search
  → 1-hop graph traversal
  → deduplication
  → RETURN candidates (NO touch, NO renew)

score_candidates (graph.rs)
  → compute decay_score
  → compute usage bonus using usage_count (always 0 since never touched)
  → sort descending

enforce_budget (types.rs)
  → greedy selection within token budget
  → RETURN ranked memories (NO renew call)
```

**Both `usage_count` and `renewal_count` are never written.** The `touch_*` functions exist and the `renew_memory_bypass` function exists, but neither is called from the retrieval pipeline.

#### Key Code Locations (current, may shift)
- `retrieve_candidates`: sqlite.rs:2037
- `retrieve_for_context`: sqlite.rs:2176
- `renew_memory_bypass`: sqlite.rs:1272
- `touch_entity` / `touch_edge`: sqlite.rs:1076/1090
- `Graph::renew_memory`: graph.rs:697
- `score_candidate`: types.rs:651

#### Implementation Plan

**Step 1 — Wire touch into retrieval (small, safe)**
- In `retrieve_candidates`, after building `cand_map`, call `touch_entity(id)` for each entity candidate and `touch_edge(id)` for each edge candidate
- This activates the `usage_count` and `last_recalled_at` fields so scoring bonus works
- No new fields, no migrations needed

**Step 2 — Add renewal_count column (migration 009)**
- Add `renewal_count INTEGER DEFAULT 0` to `entities` and `edges`
- Migration must be idempotent

**Step 3 — Add max_renewals to AgentPolicy**
- `AgentPolicy.max_renewals: usize = 5`

**Step 4 — Implement gated renew_memory**
- `renew_memory(id, memory_type, max_renewals)`:
  1. Check memory_type is Fact or Preference → otherwise return false
  2. Check current `renewal_count < max_renewals` → otherwise return false
  3. Reset `created_at` to now + set TTL
  4. Increment `renewal_count`
  5. Return true

**Step 5 — Call renew_memory on budget-enforced results**
- After `enforce_budget`, iterate returned memories
- For each with type Fact or Preference, call `renew_memory`
- Only candidates that survived budget enforcement get renewed

#### Relationship to Cleanup (A6)
- Cleanup (`cleanup_expired`) runs every 100 queries if last run > 24h ago
- Renewal extends TTL on actively-used memories; cleanup deletes unused expired content
- These are complementary: renewal keeps good memories alive; cleanup removes stale ones
- 6-month hard cutoff enforced by cleanup regardless of renewal count
