# decay_score Enhancement Plan

**File:** `crates/ctxgraph-core/src/types.rs:88`  
**Status:** Proposed  
**Date:** 2026-04-07

---

## Current Signature

```rust
pub fn decay_score(
    &self,
    base_confidence: f64,    // always passed as 1.0 — dead parameter
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
) -> f64
```

Internal behavior:
```
Fact / Decision  → exponential, half_life = ttl * 0.5
Preference       → exponential, half_life = ttl * 0.7
Experience       → linear, drops to 0.0 at TTL
Pattern          → constant, returns base_confidence
Hard cutoff:     age > ttl → 0.0
```

---

## Problems

### Effectiveness

| ID | Problem | Evidence |
|----|---------|----------|
| E1 | **Recall-agnostic** — a memory recalled 50 times decays identically to one never recalled. `usage_count` and `last_recalled_at` exist on entities/edges but are ignored. Usage only compensates at scoring time via `score_candidate`, it never slows the decay curve itself. | `sqlite.rs:1287,1345` hardcode `base_confidence=1.0`, `decay_score` takes no usage/recall params |
| E2 | **Hard cliff at TTL** — a Fact at `ttl - 1s` scores ≈ 0.25; at `ttl + 1s` it scores 0.0. A 25-point instant drop. Borderline memories vanish abruptly, making grace period effectively meaningless since the memory is already dead before cleanup runs. | `types.rs:114` — `if age_secs > ttl_secs { return 0.0 }` |
| E3 | **`base_confidence` is a dead parameter** — every production call site passes `1.0`. The parameter adds confusion with zero signal. | `sqlite.rs:1287`, `sqlite.rs:1345`, all tests pass `1.0` |
| E4 | **Fact and Decision share identical curves** — tests assert they must be identical, but semantically they are not. A Fact is empirical data that should decay gradually. A Decision is an architectural commitment that should stay fully trusted for most of its life, then drop sharply near the review threshold. | `types.rs:119` — `MemoryType::Fact \| MemoryType::Decision =>` same arm |
| E5 | **Exponential decay mismodels long-term memory** — exponential assumes a constant, time-independent forgetting rate. Empirical memory research (Wixted 2004, ACT-R) shows human declarative memory follows a power-law: rapid initial drop, slow long-tail. Exponential over-predicts long-term forgetting of foundational facts and under-predicts short-term forgetting of stale details. | Wixted (2004), ACT-R architecture |

### Performance

| ID | Problem | Evidence |
|----|---------|----------|
| P1 | **`Utc::now()` called per candidate** — `rank_candidates` calls `decay_score` for every candidate (50–200 in a normal retrieval). Each call is a separate syscall. Within one ranking pass, `now` is constant. | `graph.rs:556` — `score_candidate` → `decay_score` in a loop |
| P2 | **`get_stale_memories` fetches all TTL rows** — loads every entity/edge with a TTL into Rust before filtering by decay score. No minimum-age WHERE clause in SQL. Fresh memories (first 30% of TTL) are fetched, decay-computed, and discarded. | `sqlite.rs:1256–1262` — no age pre-filter |
| P3 | **Duplicated ranking logic** — `Graph::rank_candidates` and `Storage::retrieve_for_context` both inline the same `score → filter → sort` loop. | `graph.rs:553–563` vs `sqlite.rs:2393–2403` |

---

## Proposed Final Signature

```rust
pub fn decay_score(
    &self,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,
    usage_count: u32,
    last_recalled_at: Option<DateTime<Utc>>,
) -> f64
```

`base_confidence` is removed (always `1.0`). If extraction-time confidence becomes useful in future, it belongs on `RetrievalCandidate` where `score_candidate` can multiply it externally — keeping `decay_score` focused on temporal freshness only.

---

## Improvements

### Change 1: Pure function — pass `now` externally (P1)

Remove the internal `Utc::now()` call. Callers capture `now` once per batch:

```rust
// sqlite.rs — before the loop
let now = Utc::now();
for row in rows {
    let score = memory_type.decay_score(created_at, ttl, now, usage_count, last_recalled_at);
}
```

`score_candidate` captures `now` once and passes it through `rank_candidates`. Eliminates N syscalls per ranking pass and makes the function deterministic (same inputs → same output), which is critical for testing.

---

### Change 2: Recall-aware decay — usage_count + last_recalled_at (E1)

Two signals work together:

**Frequency boost** — frequently recalled memories get an extended effective TTL, capped at 1.5x:

```rust
// recall_boost: 1.0 at usage=0, 1.5 at usage=20, capped
let recall_boost = 1.0 + 0.025 * (usage_count as f64).min(20.0);
let effective_ttl = ttl_secs * recall_boost;
```

**Recency bonus** — a memory recalled within the last 7 days gets a flat +0.10 additive bonus on the final score:

```rust
let recency_bonus = last_recalled_at
    .map(|lr| if (now - lr).num_days() < 7 { 0.10 } else { 0.0 })
    .unwrap_or(0.0);

let score = (raw_decay * recall_boost + recency_bonus).min(1.0);
```

**Why this formula over origin-reset:** Resetting age to `last_recalled_at` is too aggressive — a memory recalled once yesterday would appear 1-day-old regardless of 60-day history. Extending the effective TTL is conservative: a memory recalled 20 times can live up to 1.5x its nominal TTL, but it still expires.

**Example impact:**
- Fact, 90d TTL, age 60d, recalled 0 times → score ≈ 0.40
- Fact, 90d TTL, age 60d, recalled 20 times → score ≈ 0.60 (effective TTL = 135d)
- Fact, 90d TTL, age 60d, recalled 5 times 3 days ago → score ≈ 0.55 + 0.10 recency

**Cleanup stays conservative:** `cleanup_expired` should continue to use hard TTL + grace period, not recall-aware effective TTL. Cleanup has higher blast radius than ranking. Keep them separate until recall-aware ranking is validated.

---

### Change 3: Soft expiration tail — remove the hard cliff (E2)

Replace the binary cliff with a fast-decaying tail that starts at the TTL value and approaches zero asymptotically:

```rust
// Before — hard cliff:
if age_secs > ttl_secs {
    return 0.0;
}

// After — soft tail past TTL:
if age_secs > ttl_secs {
    let overshoot = (age_secs - ttl_secs) / ttl_secs;
    return (0.25 * (-3.0 * overshoot).exp()).max(0.0);
}
```

The tail anchors at `0.25` at exactly `age = ttl` (matching the Fact/Decision exponential value at expiry), then decays steeply:

| Age past TTL | Tail score |
|---|---|
| TTL + 0% | 0.250 |
| TTL + 50% | 0.056 |
| TTL + 100% | 0.012 |
| TTL + 150% | 0.003 |

Effectively invisible by `2 × TTL`. `Experience` is unaffected — linear already reaches `0.0` at TTL naturally. This makes the grace period meaningful: a memory continues to decay naturally through the 7-day grace window rather than arriving at cleanup already dead.

---

### Change 4: Sigmoid decay for Decision (E4)

Decision memories represent architectural commitments. They should stay trusted for most of their TTL (the "still valid" period), then drop sharply as they approach the re-verify threshold. An exponential curve with `half_life = ttl * 0.5` drops to 0.5 at day 45 of a 90-day TTL — too early.

Add a new private function:

```rust
/// Sigmoid decay: stays near 1.0 through most of the TTL,
/// drops sharply after the inflection point at 80% of TTL.
///
/// f(t) = 1 / (1 + exp(k * (t - inflection)))
/// where t = age / ttl, k = 10 (steepness), inflection = 0.8
fn decay_sigmoid(age_secs: f64, ttl_secs: f64) -> f64 {
    let k = 10.0;
    let inflection = 0.8;
    let t = age_secs / ttl_secs;
    1.0 / (1.0 + (k * (t - inflection)).exp())
}
```

Split the match arm:

```rust
// Before:
MemoryType::Fact | MemoryType::Decision => {
    let half_life = ttl_secs * 0.5;
    base_confidence * decay_exponential(age_secs, half_life)
}

// After:
MemoryType::Fact => {
    let half_life = ttl_secs * 0.5;
    decay_exponential(age_secs, half_life)
}
MemoryType::Decision => {
    decay_sigmoid(age_secs, ttl_secs)
}
```

**Score profile for a 90-day Decision TTL:**

| Age | Sigmoid (proposed) | Exponential (current) |
|-----|---|---|
| Day 0  | ~1.00 | 1.00 |
| Day 10 | ~0.98 | 0.86 |
| Day 50 | ~0.88 | 0.54 |
| Day 72 | ~0.50 (inflection) | 0.33 |
| Day 82 | ~0.12 | 0.19 |
| Day 90 | ~0.02 | 0.25 → cliff |

The sigmoid keeps decisions fully trusted for longer, then delivers a clear "this needs review" signal in the final 20% of TTL — directly supporting Phase C re-verification in CLAUDE.md.

The existing test `test_decay_fact_and_decision_identical` must be removed and replaced with separate curve tests.

---

### Change 5: Power-law decay for Fact (E5) — deferred

The cognitively grounded improvement: replace exponential with power-law for Fact and Preference.

```rust
/// Power-law decay: (1 + age/half_life)^(-alpha)
///
/// Intersects exponential at half_life (both = 0.5).
/// Faster initial drop, slower long-tail than exponential.
fn decay_power_law(age_secs: f64, half_life_secs: f64, alpha: f64) -> f64 {
    (1.0 + age_secs / half_life_secs).powf(-alpha)
}
```

With `alpha = 1.0`, power-law and exponential intersect at `half_life` (both = 0.5). Power-law is then:
- More aggressive in the first half (stale details drop faster)
- More generous in the second half (consolidated facts persist longer)

**Why deferred:** This changes scores across all existing Fact and Preference memories. All 19 decay tests need recalibration with specific `alpha` values derived from observed recall patterns. Should be done in a separate pass after Changes 1–4 are validated.

---

### Change 6: SQL age pre-filter in `get_stale_memories` (P2)

Avoid loading fresh memories into Rust just to discard them. Add a minimum age filter before applying `LIMIT/OFFSET`:

```sql
WHERE ttl_seconds IS NOT NULL
  AND ttl_seconds > 0
  AND unixepoch(created_at) < unixepoch('now') - (ttl_seconds / 3)
```

This skips memories in the first third of their TTL where `decay_score > 0.85` — no value computing or checking those. Keep the Rust `decay_score` call as the exact final filter.

Note: use `unixepoch()` (SQLite native integer function) rather than `strftime('%s', ...)` to avoid string conversion overhead.

---

### Change 7: Deduplicate ranking logic (P3)

Extract the shared `score → filter expired → sort descending` pipeline into one function, callable by both `Graph::rank_candidates` (`graph.rs:553`) and `Storage::retrieve_for_context` (`sqlite.rs:2393`):

```rust
pub fn rank_scored_candidates(candidates: Vec<RetrievalCandidate>, now: DateTime<Utc>) -> Vec<ScoredCandidate> {
    let mut scored: Vec<ScoredCandidate> = candidates
        .into_iter()
        .map(|c| ScoredCandidate { composite_score: score_candidate(&c, now), candidate: c })
        .filter(|s| s.composite_score > 0.0)
        .collect();
    scored.sort_by(|a, b| b.composite_score.partial_cmp(&a.composite_score).unwrap_or(Equal));
    scored
}
```

---

## Implementation Order

Ship all breaking signature changes (Changes 1, 2, 3, 4) in a single migration — one change, not four.

| Phase | Changes | Breaks? | Risk |
|-------|---------|---------|------|
| **Phase 1** | Change 1 (pure `now`) + Change 3 (soft tail) + Change 4 (sigmoid Decision) — refactor signature, fix cliff, differentiate Decision | Yes — all `decay_score` call sites | Low — curves shift slightly, no feature regression |
| **Phase 2** | Change 2 (recall-aware) — add `usage_count` + `last_recalled_at` to signature and `RetrievalCandidate` | Yes — add field to struct, update SQL queries | Medium — behavioral change in retrieval ranking |
| **Phase 3** | Change 6 (SQL pre-filter) + Change 7 (dedup ranking) | No | Low — SQL optimization, maintainability |
| **Phase 4** | Change 5 (power-law) | Yes — all decay tests recalibrate | Medium — requires benchmark validation against real data |

---

## Files Affected

| File | Changes |
|------|---------|
| `crates/ctxgraph-core/src/types.rs` | `decay_score` signature, `decay_exponential`, `decay_linear`, new `decay_sigmoid`, `score_candidate`, `RetrievalCandidate` struct |
| `crates/ctxgraph-core/src/storage/sqlite.rs` | `get_stale_memories` SQL + Rust, `cleanup_expired`, `retrieve_for_context` |
| `crates/ctxgraph-core/src/graph.rs` | `rank_candidates` |
| `crates/ctxgraph-core/tests/core_tests.rs` | All A2 decay tests (19 call sites) |

## Call Sites

| File | Line | Current | Update |
|------|------|---------|--------|
| `types.rs` | 601 | `decay_score(candidate.base_confidence, ...)` | Remove `base_confidence`, add `now`, `usage_count`, `last_recalled_at` from candidate |
| `sqlite.rs` | 1287 | `decay_score(1.0, created_at, ttl)` | Remove `1.0`, add `now`, `usage_count`, `last_recalled_at` from entity row |
| `sqlite.rs` | 1345 | `decay_score(1.0, recorded_at, ttl)` | Same as above for edge row |
| `core_tests.rs` | 781,794,808,819,824,834,847,860,870,884,885,900,902,906,909,916,924,926,947 | `decay_score(1.0, created_at, ttl)` | Remove `1.0`, add fixed `now = Utc::now()`, `0u32`, `None` |

---

## References

- Wixted, J. T. (2004). *The psychology and neuroscience of forgetting.* Annual Review of Psychology.
- Rubin, D. C., & Wenzel, A. E. (1996). *One hundred years of forgetting: A quantitative description of retention.* Psychological Review.
- ACT-R cognitive architecture: http://act-r.psy.cmu.edu/
- FSRS spaced repetition: https://github.com/open-spaced-repetition/fsrs4anki/wiki/The-Algorithm
- SQLite date functions: https://www.sqlite.org/lang_datefunc.html
