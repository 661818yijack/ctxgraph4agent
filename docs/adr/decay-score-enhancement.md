# Decay Score Enhancement

## Status

Proposed

## Context

`MemoryType::decay_score` (`types.rs:88`) is the core freshness signal driving ranking, stale detection, and cleanup. Analysis of all call sites revealed 3 effectiveness gaps and 3 performance issues.

## Problem Analysis

### Effectiveness Gaps

| # | Issue | Evidence |
|---|-------|----------|
| E1 | **Recall-agnostic decay** — a memory recalled 50 times decays identically to one never recalled. `usage_count` and `last_recalled_at` exist on entities/edges but are ignored by `decay_score`. Only `score_candidate` applies a usage bonus, meaning decay_score alone is a weak relevance signal. | `sqlite.rs:1287` hardcodes `base_confidence=1.0`, `types.rs:88` has no usage/recall params |
| E2 | **Hard cliff at TTL** — a Fact at TTL-1s scores 0.25, at TTL+1s scores 0.0. No soft tail. Borderline memories vanish abruptly. | `types.rs:114` — `if age_secs > ttl_secs { return 0.0 }` |
| E3 | **`base_confidence` is dead** — every call site passes `1.0`. The parameter adds complexity with zero signal. | `sqlite.rs:1287,1345`, all tests pass `1.0` |

### Performance Gaps

| # | Issue | Evidence |
|---|-------|----------|
| P1 | **`Utc::now()` per candidate** — `rank_candidates` calls `decay_score` for every candidate (50-200), each doing a system clock syscall. Within a single ranking pass, `now` is constant. | `graph.rs:556` — `score_candidate` → `decay_score` in a loop |
| P2 | **SQL fetches all TTL entities for stale check** — `get_stale_memories` loads every entity with a TTL, computes decay in Rust, then filters. Could pre-filter by age in SQL. | `sqlite.rs:1256-1262` — no age WHERE clause |
| P3 | **Duplicated ranking logic** — `Graph::rank_candidates` and `Storage::retrieve_for_context` both inline the same `score_candidate` loop. | `graph.rs:553-563` vs `sqlite.rs:2393-2403` |

## Proposed Changes

### 1. Add `now` parameter (P1)

Pass `DateTime<Utc>` into `decay_score` instead of calling `Utc::now()` internally. The caller captures `now` once per batch.

```rust
pub fn decay_score(
    &self,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,
) -> f64
```

`score_candidate` captures `Utc::now()` once, passes it through. `get_stale_memories` captures once, passes to all iterations. Eliminates N syscalls per ranking pass.

### 2. Recall-aware decay (E1)

Add `usage_count` and `last_recalled_at` as optional params. Frequently-recalled memories decay slower:

```rust
pub fn decay_score(
    &self,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,
    usage_count: u32,
    last_recalled_at: Option<DateTime<Utc>>,
) -> f64 {
    let raw = self.compute_raw_decay(created_at, ttl, now);

    // Recall boost: cap at 1.5x for usage_count >= 20
    let recall_boost = 1.0 + 0.025 * (usage_count as f64).min(20.0);

    // Recency bonus: small bump if recalled within last 7 days
    let recency_bonus = last_recalled_at
        .map(|lr| if (now - lr).num_days() < 7 { 0.1 } else { 0.0 })
        .unwrap_or(0.0);

    (raw * recall_boost + recency_bonus).min(1.0)
}
```

A Fact recalled 20 times in the last week decays at 1.5x its raw rate — effectively extending its useful life by ~50%. A never-recalled Fact decays at the base rate.

### 3. Soft expiration tail (E2)

Replace the hard `return 0.0` cliff with a fast-decaying tail:

```rust
if age_secs > ttl_secs {
    let overshoot = (age_secs - ttl_secs) / ttl_secs;
    return 0.25 * (-3.0 * overshoot).exp();
}
```

At TTL: tail starts at 0.25 (same as exponential decay value at TTL). At 1.5x TTL: 0.25 * e^(-1.5) ≈ 0.056. At 2x TTL: 0.25 * e^(-3) ≈ 0.012. Effectively invisible by 2x TTL, but prevents the cliff.

### 4. Remove `base_confidence` param (E3)

Delete the parameter. It's always 1.0. If needed later, it can be added back as a field on `RetrievalCandidate` that `score_candidate` multiplies externally — keeping `decay_score` focused on temporal freshness only.

### 5. SQL-side age pre-filter (P2)

In `get_stale_memories`, add a minimum age filter to reduce rows fetched:

```sql
WHERE ttl_seconds IS NOT NULL
  AND ttl_seconds > 0
  AND created_at < datetime('now', '-' || (?1 / 3) || ' seconds')
```

Where `?1` is the longest TTL (7,776,000s for 90d). This skips the first third of any memory's life where decay_score > 0.85 — no need to compute or check those.

### 6. Deduplicate ranking logic (P3)

Extract the `score → filter → sort` pipeline into a shared function callable by both `Graph::rank_candidates` and `Storage::retrieve_for_context`, eliminating the copy-paste at `sqlite.rs:2393-2403`.

## Impact Summary

| Change | Effectiveness | Performance | Breaking |
|--------|:---:|:---:|:---:|
| `now` parameter | — | Eliminates N syscalls/batch | Yes (sig change) |
| Recall-aware decay | Frequently-used memories rank higher | Negligible (2 multiplies) | Yes (sig change) |
| Soft expiration tail | No cliff edge, smoother ranking | Negligible | No (internal) |
| Remove `base_confidence` | Simpler API, less confusion | Negligible | Yes (sig change) |
| SQL age pre-filter | — | Fewer rows fetched, fewer decay calls | No (SQL only) |
| Dedup ranking logic | — | Maintainability | No (refactor) |

All three breaking changes (1, 2, 4) are in a single signature refactor — one migration, not three. The `RetrievalCandidate` struct already has `usage_count`; it just needs `last_recalled_at` added.

## New Signature

```rust
impl MemoryType {
    pub fn decay_score(
        &self,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
        now: DateTime<Utc>,
        usage_count: u32,
        last_recalled_at: Option<DateTime<Utc>>,
    ) -> f64 { ... }
}
```

## Files Affected

- `crates/ctxgraph-core/src/types.rs` — `decay_score`, `decay_exponential`, `decay_linear`, `score_candidate`, `RetrievalCandidate`
- `crates/ctxgraph-core/src/storage/sqlite.rs` — `get_stale_memories`, `retrieve_for_context`, `cleanup_expired`
- `crates/ctxgraph-core/src/graph.rs` — `rank_candidates`
- `crates/ctxgraph-core/tests/core_tests.rs` — all A2 decay tests
- `crates/ctxgraph-core/src/types.rs` (tests) — inline unit tests

## Call Sites to Update

| File | Line | Current Call | Notes |
|------|------|-------------|-------|
| `types.rs` | 601 | `memory_type.decay_score(candidate.base_confidence, ...)` | `score_candidate` — capture `now` once, pass `candidate.usage_count` |
| `sqlite.rs` | 1287 | `memory_type.decay_score(1.0, created_at, ttl)` | `get_stale_memories` entities — capture `now`, add usage/recall from query |
| `sqlite.rs` | 1345 | `memory_type.decay_score(1.0, recorded_at, ttl)` | `get_stale_memories` edges — same |
| `core_tests.rs` | 781,794,808,819,824,834,847,860,870,884,885,900,902,906,909,916,924,926,947 | `mt.decay_score(1.0, created_at, ttl)` | Update all 19 test calls |
