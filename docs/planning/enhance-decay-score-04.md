# Enhance `MemoryType::decay_score`

**File:** `crates/ctxgraph-core/src/types.rs:88`
**Status:** Proposed
**Date:** 2026-04-07

---

## Problem Statement

`decay_score` is the core freshness signal for ranking, stale detection, and cleanup. Analysis of all call sites identified 4 effectiveness issues and 3 performance issues.

### Effectiveness Issues

| # | Issue | Evidence |
|---|-------|----------|
| E1 | **Recall-agnostic decay** — a memory recalled 50 times decays identically to one never recalled. `usage_count` and `last_recalled_at` exist on entities/edges but are ignored by `decay_score`. Only `score_candidate` applies a post-hoc usage bonus. | `sqlite.rs:1287` hardcodes `base_confidence=1.0`; `types.rs:88` has no usage/recall params |
| E2 | **Hard cliff at TTL** — a Fact at TTL-1s scores 0.25, at TTL+1s scores 0.0. No soft tail. Borderline memories vanish abruptly. | `types.rs:114` — `if age_secs > ttl_secs { return 0.0 }` |
| E3 | **`base_confidence` is unwired, not dead** — every call site passes `1.0` because the signal was never connected. `edges.confidence` already varies (GLiREL/LLM: 0.45–0.95) and represents extraction quality. Wiring edge candidates to use it adds a real discriminating signal with no schema changes. | `sqlite.rs:1287,1345` hardcode `1.0`; `edges.confidence` already stored |
| E4 | **Decision and Fact have identical curves** — but semantically they differ. A Decision is an architectural commitment that should stay near full confidence for most of its TTL, then drop sharply near review time. Exponential decay is the wrong shape. | `core_tests.rs:884` — test asserts `fact_score == decision_score` |

### Performance Issues

| # | Issue | Evidence |
|---|-------|----------|
| P1 | **`Utc::now()` per candidate** — `rank_candidates` calls `decay_score` for every candidate (50-200), each doing a system clock syscall. `now` is constant within a single ranking pass. | `graph.rs:556` |
| P2 | **SQL fetches all TTL entities for stale check** — `get_stale_memories` loads every entity with a TTL, computes decay in Rust, then filters. Could pre-filter by age in SQL. | `sqlite.rs:1256-1262` — no age WHERE clause |
| P3 | **Duplicated ranking logic** — `Graph::rank_candidates` and `Storage::retrieve_for_context` both inline the same `score_candidate` loop. | `graph.rs:553-563` vs `sqlite.rs:2393-2403` |

---

## Proposed Changes

### Change 1: Pure function — pass `now` (P1)

Replace the internal `Utc::now()` call with an explicit `now` parameter. The caller captures `now` once per batch and passes it down.

```rust
// Three-tier delegation for backward compatibility:
pub fn decay_score(...) -> f64                          // convenience wrapper, calls decay_score_at
pub fn decay_score_at(...) -> f64                       // time-injected, calls decay_score_with_usage_at
pub fn decay_score_with_usage_at(...) -> f64            // full implementation with recall awareness
```

`score_candidate` captures `Utc::now()` once. `get_stale_memories` captures once before the loop. Eliminates N syscalls per ranking pass and makes the function deterministic for testing.

> **Important:** `get_stale_memories` calls `decay_score_at` (non-recall-aware), not `decay_score_with_usage_at`. Stale detection and cleanup stay on the original hard-TTL logic until ranking is validated. Cleanup has higher blast radius than ranking — do not couple them. The SQL candidate queries for stale memories also do not select `usage_count` or `last_recalled_at`.

### Change 2: Recall-aware decay via effective age (E1)

Use `last_recalled_at` to reset the age origin. A memory recalled recently should behave like a younger memory. This directly implements the implicit re-verification goal from CLAUDE.md.

```rust
let effective_origin = last_recalled_at
    .filter(|&r| r > created_at)
    .unwrap_or(created_at);
let age_secs = (now - effective_origin).num_seconds().max(0) as f64;
```

Use `usage_count` to extend the effective half-life. Frequently recalled memories decay slower — the spacing effect from cognitive science (ACT-R, FSRS).

```rust
let recall_boost = (1.0 + 0.15 * (usage_count as f64).ln_1p()).min(1.75);
let effective_half_life = half_life * recall_boost;
```

**Examples (Fact, 90d TTL):**

| Scenario | Age | Usage | Recalled | Effective Age | Score |
|----------|-----|-------|----------|---------------|-------|
| Fresh, never used | 1d | 0 | never | 1d | ~0.99 |
| Old, never used | 60d | 0 | never | 60d | ~0.40 |
| Old, recalled yesterday | 60d | 0 | 1d ago | 1d | ~0.99 |
| Old, used 20 times | 60d | 20 | never | 60d | ~0.57 |
| Old, used 20 times, recalled yesterday | 60d | 20 | 1d ago | 1d | ~0.99 |

**Caps ensure memories still expire:** `recall_boost` is capped at 1.75x — a 90d TTL becomes at most ~157d effective. Unrecalled memories decay at the base rate. Expired unrecalled memories still score 0.0.

### Change 3: Soft expiration tail (E2)

Replace the hard `return 0.0` cliff with a fast-decaying tail beyond TTL:

```rust
// For Fact/Preference/Decision — computed per-type from actual TTL boundary score
if age_secs > ttl_secs {
    let overshoot = (age_secs - ttl_secs) / ttl_secs;
    let ttl_score = /* score at TTL for this type's curve */;
    return ttl_score * (-3.0 * overshoot).exp();
}
```

| Type | Score at TTL | Soft tail behavior |
|------|-------------|-------------------|
| Fact (exp, half=ttl/2) | 0.25 | 0.25 → 0.056 at 1.5×TTL → 0.012 at 2×TTL |
| Preference (exp, half=ttl×0.7) | 0.36 | 0.36 → 0.081 at 1.5×TTL → 0.018 at 2×TTL |
| Decision (sigmoid, k=20) | 0.018 | 0.018 → 0.004 at 1.5×TTL → 0.001 at 2×TTL |
| **Experience (linear)** | **0.00** | **No soft tail — linear already reaches 0.0 at TTL** |

Experience is the exception: its linear decay `max(0, 1 - age/ttl)` already reaches exactly 0.0 at TTL, so there is no cliff to soften. The soft tail only applies to types whose curve is still positive at the TTL boundary.

| Age | Current | Proposed (Fact) |
|-----|---------|----------|
| TTL - 1s | 0.25 | 0.25 |
| TTL + 1s | **0.00** (cliff) | 0.25 |
| 1.5x TTL | 0.00 | 0.056 |
| 2x TTL | 0.00 | 0.012 |

Effectively invisible by 2x TTL, but prevents the abrupt cliff. Grace period becomes meaningful because decay continues naturally during it.

### Change 4: Sigmoid decay for Decision (E4)

Decisions are architectural commitments. They need a plateau curve: stay near full confidence for most of TTL, then drop sharply near the end (the "needs review" signal).

```rust
fn decay_sigmoid(age_secs: f64, ttl_secs: f64) -> f64 {
    let steepness = 20.0;  // tuned for sharper drop near TTL boundary
    let inflection = 0.8;
    let t = age_secs / ttl_secs;
    1.0 / (1.0 + (steepness * (t - inflection)).exp())
}
```

| Age | Current (exponential) | Proposed (sigmoid, k=20) |
|-----|----------------------|-------------------------|
| Day 0 | 1.00 | 1.00 |
| Day 50 | 0.54 | **0.96** |
| Day 72 | 0.33 | 0.50 (inflection) |
| Day 82 | 0.22 | **0.07** |
| Day 90 | 0.25 → 0.00 (cliff) | 0.018 |

The sigmoid keeps decisions fully trusted longer, then gives a clear "needs review" signal. Steepness k=20 (vs k=10 in earlier drafts) produces a sharper drop — by day 82 the score is already 0.07, making the "review needed" signal unambiguous. The existing test asserting `fact_score == decision_score` must be replaced with separate curve tests.

### Change 5: Wire `base_confidence` from `edges.confidence` (E3)

Do not remove this parameter — wire it up properly. Callers pass `1.0` because the signal was never connected, not because it has no value. `edges.confidence` already exists in the schema and already varies.

- For `RetrievalCandidate` from **entities**: keep `base_confidence = 1.0` (no stored extraction confidence — preserves current behavior)
- For `RetrievalCandidate` from **edges**: set `base_confidence = edges.confidence`

Apply a bounded transform in `score_candidate` to guard against noisy model calibration:

```rust
// Maps [0.0, 1.0] → [0.5, 1.0]: low-confidence edges demoted but not erased
let confidence_weight = 0.5 + 0.5 * candidate.base_confidence.clamp(0.0, 1.0);
let score = decay * confidence_weight * normalized_fts * usage_bonus;
```

Use raw multiplication only if benchmarks confirm GLiREL/LLM confidence is well-calibrated. Default to the bounded transform until then.

### Change 6: SQL-side age pre-filter (P2)

In `get_stale_memories`, add a minimum age filter to reduce rows fetched from SQLite:

```sql
WHERE ttl_seconds IS NOT NULL
  AND ttl_seconds > 0
  AND created_at < datetime('now', '-' || (?1 / 3) || ' seconds')
```

Where `?1` is the longest TTL (7,776,000s for 90d). This skips the first third of any memory's life where decay_score > 0.85 — no need to fetch or compute those. Keep the Rust-side `decay_score` check as the exact final filter.

### Change 7: Deduplicate ranking logic (P3)

Extract the `score → filter → sort` pipeline into a shared function callable by both `Graph::rank_candidates` and `Storage::retrieve_for_context`, eliminating the copy-paste at `sqlite.rs:2393-2403`.

---

## API Signatures

### Public surface (non-breaking additions)

```rust
impl MemoryType {
    /// Convenience wrapper — delegates to decay_score_at(Utc::now()).
    /// Backward compatible — existing callers unchanged.
    pub fn decay_score(
        &self,
        base_confidence: f64,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
    ) -> f64;

    /// Deterministic decay with explicit time anchor.
    /// Delegates to decay_score_with_usage_at(..., None, 0, now).
    pub fn decay_score_at(
        &self,
        base_confidence: f64,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
        now: DateTime<Utc>,
    ) -> f64;

    /// Full recall-aware decay — frequently recalled memories decay slower.
    /// This is the canonical implementation; all other methods delegate here.
    pub fn decay_score_with_usage_at(
        &self,
        base_confidence: f64,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
        last_recalled_at: Option<DateTime<Utc>>,
        usage_count: u32,
        now: DateTime<Utc>,
    ) -> f64;
}
```

### Composite scoring

```rust
pub fn score_candidate(candidate: &RetrievalCandidate) -> f64;
pub fn score_candidate_at(candidate: &RetrievalCandidate, now: DateTime<Utc>) -> f64;
```

### Shared ranking (deduplicates graph.rs and sqlite.rs)

```rust
pub fn rank_scored_candidates_at(
    candidates: Vec<RetrievalCandidate>,
    now: DateTime<Utc>,
) -> Vec<ScoredCandidate>;
```

---

## Implementation Order

The phasing axis is **"can I validate this change in isolation"**, not breaking-vs-non-breaking. Each batch is a clean hypothesis: if tests fail, the cause is unambiguous.

### Batch 0 — Testability only, zero semantic change

*Validate: existing tests pass with identical scores*

| Step | Breaking? | Effort | Risk |
|------|:---------:|:------:|:----:|
| Add `decay_score_at`, route existing `decay_score` through it | No | Low | None |
| Update batch callers to capture `now` once per loop | No | Low | None |
| Update tests to use fixed `now` for deterministic assertions | No | Low | None |

Nothing moves in the score space. Pure refactor. This is the prerequisite that makes every subsequent batch measurable — if a later batch breaks a test, you know it is not the `now` change.

### Batch 1 — Isolated single-type changes

*Validate each independently: before/after score tables per type*

| Step | Breaking? | Effort | Risk |
|------|:---------:|:------:|:----:|
| Soft expiration tail | No | Low | Low |
| Sigmoid for Decision | Yes (test change) | Low | Low |

These two land together because their blast radius does not overlap:

- **Soft tail** only affects `age > ttl`. In-TTL scores are completely unchanged. Validate by confirming no in-TTL memory shifts rank.
- **Sigmoid for Decision** only affects `MemoryType::Decision`. Validate by running the Decision test set and comparing score profiles. All other types are untouched.

Neither touches stale detection or cleanup. The existing test `test_decay_fact_and_decision_identical` is removed and replaced with separate curve tests.

### Batch 2 — Recall-aware, ranking only

*Validate: recalled memories rank higher; unrecalled memories unchanged; cleanup untouched*

| Step | Breaking? | Effort | Risk |
|------|:---------:|:------:|:----:|
| Add `last_recalled_at` to `RetrievalCandidate` + populate from SQL | Yes (struct change) | Medium | Medium |
| Add `decay_score_with_usage_at` | No | Medium | Medium |
| Update `score_candidate` to use recall-aware variant | No | Low | Medium |

**Scope boundary:** update `score_candidate` only — not `get_stale_memories`, not `cleanup_expired`. Stale detection and cleanup stay on the original hard-TTL logic until ranking is validated. Cleanup has higher blast radius than ranking; do not couple them in the same batch.

The `RetrievalCandidate` struct already has `usage_count`; it just needs `last_recalled_at` added and populated from the candidate retrieval query.

### Batch 3 — API cleanup and performance

*Validate: bounded edge-ranking change plus performance metrics*

| Step | Breaking? | Effort | Risk |
|------|:---------:|:------:|:----:|
| Wire `base_confidence` from `edges.confidence` for edges; default `1.0` for entities | No | Low | Low |
| SQL age pre-filter in `get_stale_memories` | No | Low | None |
| Deduplicate ranking logic | No | Medium | Low |

No score changes for entities (default stays `1.0`). Edge ranking changes only where `edges.confidence` is already below `1.0`.

#### `base_confidence` is a real signal, not dead API

Earlier analysis noted all callers pass `1.0` and recommended removal. That was wrong. Callers pass `1.0` because the signal was never wired up — not because it has no value. `edges.confidence` already exists in the schema, already varies (GLiREL produces scores from ~0.45 to ~0.95), and already represents extraction quality. Discarding it throws away a discriminating signal the system already paid to compute.

**Why it improves ordering:** if two candidate memories match the query equally and have the same age, but one relation was extracted at confidence `0.95` and another at `0.35`, ranking them equally is wrong. `base_confidence` lets the low-confidence one be discounted.

**Wiring plan:**
- For `RetrievalCandidate` from entities: populate `base_confidence = 1.0` (no stored extraction confidence yet — preserves current behavior)
- For `RetrievalCandidate` from edges: populate `base_confidence = edges.confidence`

**Risk: noisy model calibration.** GLiREL and LLM confidence scores may not be well calibrated. Multiplying directly by raw confidence could over-penalize useful memories. Use a bounded transform if benchmarks show too much sensitivity:

```rust
// Maps confidence [0.0, 1.0] → weight [0.5, 1.0]
// Low-confidence memories are demoted but not erased.
let confidence_weight = 0.5 + 0.5 * base_confidence.clamp(0.0, 1.0);
```

**Ship with:**
- Test: same candidate, same age, same FTS score, `base_confidence=0.9` ranks above `base_confidence=0.3`
- Test: entity candidates with default `1.0` keep identical scores to current behavior
- Test: edge candidates populate `base_confidence` from `edges.confidence`
- Retrieval quality comparison before and after on a representative query set

### Failure isolation per batch

| Batch | If tests fail, the cause is... |
|-------|-------------------------------|
| 0 | The refactor — a pure mechanics error |
| 1 | Soft tail **or** sigmoid — isolated to post-TTL region or Decision type only |
| 2 | Recall-aware scoring — visible in ranking deltas, no cleanup impact |
| 3 | `base_confidence` wiring — edge ranking shifts where `edges.confidence < 1.0`; SQL/dedup changes have no score impact |

---

## Rollout Safety

- **Cleanup must remain conservative.** Do not make cleanup rely on recall-aware effective TTL until ranking behavior has been validated. Cleanup has higher blast radius than ranking. Keep hard TTL + grace-period behavior for cleanup until Batch 2 is validated.
- **Preserve backward-compatible wrapper** (optional): a `decay_score_simple(created_at, ttl)` that delegates with `usage_count=0`, `last_recalled_at=None`, `now=Utc::now()`. Allows incremental migration of call sites.

---

## Files Affected

| File | What Changes |
|------|-------------|
| `crates/ctxgraph-core/src/types.rs` | Three-tier decay delegation (`decay_score` → `decay_score_at` → `decay_score_with_usage_at`), `decay_sigmoid`, `decay_soft_tail`, bounded confidence weighting in `score_candidate_at`, `RetrievalCandidate` (+`last_recalled_at`), `rank_scored_candidates_at` (shared ranking) |
| `crates/ctxgraph-core/src/storage/sqlite.rs` | `get_stale_memories` (SQL pre-filter, `decay_score_at` non-recall-aware), `retrieve_for_context` (uses `rank_scored_candidates_at`), candidate queries (select `last_recalled_at`, wire `edge.confidence`) |
| `crates/ctxgraph-core/src/graph.rs` | `rank_candidates` (delegates to `rank_scored_candidates_at`) |
| `crates/ctxgraph-core/tests/core_tests.rs` | All 19 A2 decay test calls updated, `test_decay_fact_and_decision_identical` replaced, soft-tail and sigmoid tests added |

---

## Call Sites to Update

| File | Function | Method Called | Notes |
|------|----------|--------------|-------|
| `types.rs` | `score_candidate` | `score_candidate_at(c, Utc::now())` | Delegates to `decay_score_with_usage_at` with all params from candidate |
| `sqlite.rs` | `get_stale_memories` | `decay_score_at(1.0, created_at, ttl, now)` | Non-recall-aware — cleanup stays conservative. Does NOT select usage/recall from SQL. |
| `sqlite.rs` | `retrieve_for_context` | `rank_scored_candidates_at(candidates, now)` | Shared ranking — captures `now` once |
| `core_tests.rs` | 19 decay tests | `decay_score_at(1.0, created_at, ttl, now)` | Fixed `now`, `usage_count=0`, `last_recalled_at=None` |

---

## References

- ACT-R cognitive architecture: http://act-r.psy.cmu.edu/
- Wixted (2004), *The psychology and neuroscience of forgetting*
- FSRS spaced repetition algorithm: https://github.com/open-spaced-repetition/fsrs4anki/wiki/The-Algorithm
- CLAUDE.md: implicit re-verification — *"if a memory is recalled and used → auto-renew TTL"*
