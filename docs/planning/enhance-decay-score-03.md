# Enhancement Proposal: `MemoryType::decay_score` Effectiveness & Performance

> **File:** `crates/ctxgraph-core/src/types.rs` (lines 88–134)
> **Date:** 2026-04-07
> **Status:** Proposed
> **Supersedes:** `enhance-decay-score.md`, `decay-score-effectiveness-performance-proposal.md`, `enhancement-decay-score.md`, `decay-score-enhancement.md`

---

## Executive Summary

`MemoryType::decay_score` is the core freshness signal driving retrieval ranking, stale detection, and cleanup. Analysis of all 4 documents reveals **4 effectiveness gaps** and **3 performance issues**. This proposal consolidates the best improvements into a single, phased implementation plan.

---

## Problem Analysis

### Effectiveness Gaps

| # | Issue | Evidence |
|---|-------|----------|
| **E1** | **Recall-agnostic decay** — a memory recalled 50 times decays identically to one never recalled. `usage_count` and `last_recalled_at` exist on entities/edges but are ignored by `decay_score`. Only `score_candidate` applies a post-decay usage bonus, meaning decay alone is a weak relevance signal. | `sqlite.rs:1287` hardcodes `base_confidence=1.0`, `types.rs:88` has no usage/recall params |
| **E2** | **Hard cliff at TTL** — a Fact at TTL−1s scores ~0.25, at TTL+1s scores 0.0. Borderline memories vanish abruptly. Exponential decay is asymptotic; the hard cut is unnatural. | `types.rs:114` — `if age_secs > ttl_secs { return 0.0 }` |
| **E3** | **Fact and Decision share identical curves** — semantically different. A Fact is empirical data (gradual obsolescence). A Decision is an architectural commitment (stays valid until review, then drops sharply). | Tests assert `(fact_score - decision_score).abs() < 1e-10` |
| **E4** | **`base_confidence` is dead** — every production caller passes `1.0`. The parameter adds complexity with zero signal. | `sqlite.rs:1287,1345`, all tests pass `1.0` |

### Performance Gaps

| # | Issue | Evidence |
|---|-------|----------|
| **P1** | **`Utc::now()` per candidate** — `score_candidate` calls `decay_score` for every candidate (50–200), each doing a system clock syscall. Within a single ranking pass, `now` is constant. | `graph.rs:556` → `decay_score` in a loop |
| **P2** | **SQL fetches all TTL entities for stale check** — `get_stale_memories` loads every entity with a TTL, computes decay in Rust, then filters. Could pre-filter by age in SQL. | `sqlite.rs:1256–1262` — no age WHERE clause |
| **P3** | **Duplicated ranking logic** — `Graph::rank_candidates` and `Storage::retrieve_for_context` both inline the same `score_candidate` loop. | `graph.rs:553–563` vs `sqlite.rs:2393–2403` |

---

## Proposed Changes

### Phase 1: Correctness (Low Risk, High Value)

#### 1A. Add `now` parameter — pure function (P1)

Add a time-injected function and route the existing one through it:

```rust
/// Deterministic decay score with explicit time anchor.
///
/// Callers doing batch operations should compute `now` once
/// and pass it to all calls for consistent results.
pub fn decay_score_at(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,
) -> f64 {
    let age_secs = (now - created_at).num_seconds().max(0) as f64;
    // ... existing decay logic using age_secs instead of Utc::now()
}

/// Convenience wrapper: uses Utc::now().
pub fn decay_score(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
) -> f64 {
    self.decay_score_at(base_confidence, created_at, ttl, Utc::now())
}
```

**Benefits:**
- Tests become deterministic (pass fixed `now`)
- Batch callers compute `now` once → eliminates N syscalls
- Existing callers unchanged (backward compatible)

**Call sites:** 3 production + 19 test calls — all delegate through existing wrapper initially.

---

#### 1B. Soft expiration tail (E2)

Replace the hard `return 0.0` cliff with an exponential overshoot tail:

```rust
if age_secs > ttl_secs {
    // Soft tail: starts at exponential value at TTL, decays rapidly.
    // At TTL:     score ≈ 0.25 * e^0     = 0.25
    // At 1.5×TTL: score ≈ 0.25 * e^(-1.5) ≈ 0.056
    // At 2×TTL:   score ≈ 0.25 * e^(-3)   ≈ 0.012  (effectively gone)
    let overshoot = (age_secs - ttl_secs) / ttl_secs;
    return base_confidence * 0.25 * (-3.0 * overshoot).exp();
}
```

**Why this formula:** The exponential at TTL for a Fact (half_life = ttl/2) gives `exp(-2·ln(2)) = 0.25`. The overshoot tail extends naturally from that point. No discontinuity.

| Age | Current (hard cliff) | Proposed (soft tail) |
|-----|---------------------|---------------------|
| TTL − 1s | ~0.25 | ~0.25 (unchanged) |
| TTL | **0.00** ← cliff | ~0.25 |
| TTL + 10% | **0.00** | ~0.18 |
| 1.5× TTL | **0.00** | ~0.056 |
| 2× TTL | **0.00** | ~0.012 |

**Experience type is unaffected** — linear decay already reaches 0 at TTL, so the overshoot guard never triggers for it.

---

#### 1C. Sigmoid decay for Decision (E3)

Decisions are architectural commitments. They need a **plateau curve**: stay near full confidence for most of the TTL (the "still valid" period), then drop sharply near review time (the "needs review" signal).

```rust
/// Sigmoid decay: stays near 1.0 for most of the lifetime,
/// then drops sharply after the inflection point.
///
/// f(t) = 1 / (1 + exp(k * (t - inflection)))
/// where t = age / ttl, k = steepness, inflection = 0.8
///
/// Returns ~1.0 at age=0, ~0.5 at age=0.8×ttl, ~0.0 at age=ttl
fn decay_sigmoid(age_secs: f64, ttl_secs: f64) -> f64 {
    let k = 10.0;          // steepness
    let inflection = 0.8;   // drop starts at 80% of TTL
    let t = age_secs / ttl_secs;
    1.0 / (1.0 + (k * (t - inflection)).exp())
}
```

Split the match arm:

```rust
// Before — identical curves:
MemoryType::Fact | MemoryType::Decision => {
    let half_life = ttl_secs * 0.5;
    base_confidence * decay_exponential(age_secs, half_life)
}

// After — differentiated curves:
MemoryType::Fact => {
    let half_life = ttl_secs * 0.5;
    base_confidence * decay_exponential(age_secs, half_life)
}
MemoryType::Decision => {
    base_confidence * decay_sigmoid(age_secs, ttl_secs)
}
```

**90-day Decision TTL score profile:**

| Age | Current (exponential) | Proposed (sigmoid) |
|-----|----------------------|-------------------|
| Day 0  | 1.00 | 1.00 |
| Day 10 | 0.94 | 0.98 |
| Day 50 | 0.54 | 0.88 |
| Day 72 (80%) | 0.33 | **0.50** (inflection) |
| Day 82 | 0.25 | 0.12 |
| Day 90 | **0.00** (cliff) | 0.02 |

The sigmoid keeps decisions trusted longer, then gives a clear "review needed" signal.

---

### Phase 2: Recall-Aware Decay (Medium Risk, High Value)

#### 2A. Add recall-aware scoring variant (E1)

```rust
/// Decay score that accounts for usage history.
///
/// Frequently recalled memories decay slower — the "spacing effect"
/// from cognitive science: active recall strengthens memory traces.
pub fn decay_score_with_usage_at(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    last_recalled_at: Option<DateTime<Utc>>,
    usage_count: u32,
    now: DateTime<Utc>,
) -> f64 {
    let Some(ttl) = ttl else {
        return base_confidence; // no TTL → no decay
    };

    let ttl_secs = ttl.as_secs_f64();
    if ttl_secs == 0.0 {
        return 0.0;
    }

    // Effective TTL grows with usage (capped at 1.75× for usage_count ≥ 20)
    let recall_boost = (1.0 + 0.15 * (usage_count as f64).ln_1p()).min(1.75);
    let effective_ttl = ttl_secs * recall_boost;

    // Age: use the more recent of created_at or last_recalled_at
    let anchor = last_recalled_at.unwrap_or(created_at);
    let age_created = (now - created_at).num_seconds().max(0) as f64;
    let age_recalled = (now - anchor).num_seconds().max(0) as f64;
    // Cap effective age so old memories can't appear brand-new
    // from a single stale recall — max reduction is 25% of TTL
    let age = age_created.min(age_recalled + ttl_secs * 0.25);

    // Apply per-type curve using effective_ttl
    match self {
        MemoryType::Pattern => base_confidence,
        MemoryType::Fact | MemoryType::Decision => {
            let half_life = effective_ttl * 0.5;
            base_confidence * decay_exponential(age, half_life)
        }
        MemoryType::Preference => {
            let half_life = effective_ttl * 0.7;
            base_confidence * decay_exponential(age, half_life)
        }
        MemoryType::Experience => {
            base_confidence * (1.0 - (age / effective_ttl).min(1.0))
        }
    }
}
```

**Example:**
- Memory created 60 days ago, 90d TTL, never recalled → age 60d, score ≈ 0.40
- Memory created 60 days ago, 90d TTL, recalled 5 days ago, 10 uses → effective age ≈ 9d, effective TTL ≈ 124d, score ≈ 0.88

**Rationale:**
- Recency matters: a memory recalled yesterday should outrank one unused for months
- Frequency matters: repeated recall is a proxy for durable relevance
- Boost is capped (1.75×): frequently recalled memories still age unless renewed or converted to patterns

---

#### 2B. Add `last_recalled_at` to `RetrievalCandidate`

```rust
pub struct RetrievalCandidate {
    // ... existing fields
    pub last_recalled_at: Option<DateTime<Utc>>,  // NEW
}
```

Update SQLite candidate queries to select `last_recalled_at` for entities and edges. Update `score_candidate` to call `decay_score_with_usage_at`.

This makes the existing `touch_entity` and `touch_edge` methods materially affect future ranking, not just metadata for cleanup.

---

#### 2C. Remove dead `base_confidence` parameter (E4)

After Phase 2, remove `base_confidence` from the new signatures. All callers pass `1.0`. If source reliability becomes useful later, it can be re-added as an external multiplier in `score_candidate`.

**Option:** Defer this to Phase 3 to reduce breaking changes.

---

### Phase 3: SQL Performance (Low Risk, Medium Value)

#### 3A. Push stale-memory pre-filter into SQL (P2)

`get_stale_memories` currently loads rows ordered by `created_at DESC`, applies `LIMIT/OFFSET`, then filters by `decay_score < threshold` in Rust. Problems:

- Recent fresh rows fill the limit, missing older stale memories
- Unnecessary Rust-side decay computation on fresh rows

**Improvement:** Compute age cutoffs per memory type and filter in SQL before `LIMIT/OFFSET`.

For exponential decay, inverse the curve to get the minimum age for a given threshold:

```
threshold = exp(-ln(2) / half_life * age)
age = -ln(threshold) * half_life / ln(2)
```

For linear decay:
```
threshold = 1 - age / ttl
age = ttl * (1 - threshold)
```

SQL pre-filter:
```sql
WHERE ttl_seconds IS NOT NULL
  AND ttl_seconds > 0
  -- Skip first 1/3 of TTL where decay_score > 0.85 (no need to check)
  AND created_at < datetime('now', '-7776000 seconds')  -- 90d max TTL / 3
```

Keep the Rust `decay_score_at` check as a final exact filter.

**Why `unixepoch()`:** SQLite's `unixepoch()` returns a numeric value directly, avoiding `strftime` string conversion overhead. Use precomputed RFC3339 cutoff timestamps from Rust for the most precise filter.

---

#### 3B. Deduplicate ranking logic (P3)

Extract the `score → filter → sort` pipeline into a shared function callable by both `Graph::rank_candidates` and `Storage::retrieve_for_context`:

```rust
/// Rank candidates: score → filter expired → sort descending → enforce budget.
pub fn rank_and_budget(
    candidates: Vec<RetrievalCandidate>,
    budget_tokens: usize,
    now: DateTime<Utc>,
) -> (Vec<RankedMemory>, usize) {
    // Shared logic currently duplicated in graph.rs:553 and sqlite.rs:2393
}
```

---

## New Signatures (Final State)

After all 3 phases:

```rust
impl MemoryType {
    /// Deterministic decay with explicit time anchor.
    pub fn decay_score_at(
        &self,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
        now: DateTime<Utc>,
    ) -> f64;

    /// Recall-aware decay — frequently used memories decay slower.
    pub fn decay_score_with_usage_at(
        &self,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
        last_recalled_at: Option<DateTime<Utc>>,
        usage_count: u32,
        now: DateTime<Utc>,
    ) -> f64;

    /// Convenience wrapper (delegates to decay_score_at).
    pub fn decay_score(
        &self,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
    ) -> f64;
}
```

---

## Implementation Order

| Phase | Step | Breaking? | Effort | Risk |
|-------|------|:---------:|:------:|:----:|
| **1A** | Add `decay_score_at`, route `decay_score` through it | No | Low | None |
| **1B** | Soft expiration tail | No | Low | Low |
| **1C** | Sigmoid for Decision | Yes (test change) | Low | Low |
| **2A** | Add `decay_score_with_usage_at` | No | Medium | Medium |
| **2B** | Add `last_recalled_at` to `RetrievalCandidate` + update `score_candidate` | Yes (struct change) | Medium | Medium |
| **2C** | Remove `base_confidence` param | Yes | Low | Low |
| **3A** | SQL age pre-filter in `get_stale_memories` | No | Low | None |
| **3B** | Deduplicate ranking logic | No | Medium | Low |

**Ship in 3 batches:**
1. **Batch 1** (1A + 1B): Non-breaking, immediate value. Tests gain determinism.
2. **Batch 2** (1C + 2A + 2B + 2C): One breaking PR — all signature changes together.
3. **Batch 3** (3A + 3B): Performance-only, no behavioral change.

---

## Risk & Compatibility

| Concern | Mitigation |
|---------|-----------|
| **Cleanup blast radius** | Keep cleanup conservative — use hard TTL + grace period. Do NOT make cleanup rely on recall-aware effective TTL until ranking behavior is validated. |
| **Test churn** | Phase 1C (sigmoid) breaks `test_decay_fact_and_decision_identical`. Replace with separate tests. All other test changes are mechanical (pass `now` param). |
| **Backward compat** | Phase 1A adds `decay_score_at` without removing `decay_score`. Existing callers compile unchanged. |
| **Over-boosting** | `recall_boost` capped at 1.75×. A memory recalled 1000 times won't become immortal. The age cap (`min(age_recalled + ttl*0.25)`) prevents a single stale recall from making a 90-day-old memory appear fresh. |

---

## Research References

- **ACT-R declarative memory model:** Treats memory availability as a function of traces, recency, and frequency. Supports using `last_recalled_at` and `usage_count` as bounded reinforcement signals. — *Springer, 2023*
- **Power-law vs exponential forgetting:** Empirical human memory data consistently favors power-law (rapid initial drop, slow long-tail). Exponential systematically over-predicts long-term forgetting. — *Wixted (2004), Rubin & Wenzel (1996)*
- **FSRS spaced repetition:** Separates retrievability from stability, uses recall history to update future forgetting behavior. Too heavy for ctxgraph (no review outcomes stored), but supports the direction of recall-aware decay. — *open-spaced-repetition/fsrs4anki*
- **SQLite date functions:** `unixepoch()` returns numeric seconds directly, avoiding `strftime` string conversion overhead. — *SQLite lang_datefunc*

---

## Files Affected

| File | Changes |
|------|---------|
| `crates/ctxgraph-core/src/types.rs` | `decay_score_at`, `decay_score_with_usage_at`, `decay_sigmoid`, update `score_candidate`, `RetrievalCandidate` struct |
| `crates/ctxgraph-core/src/storage/sqlite.rs` | `get_stale_memories` (SQL pre-filter + pass usage/recall), `retrieve_for_context` (use shared ranking), candidate queries (select `last_recalled_at`) |
| `crates/ctxgraph-core/src/graph.rs` | `rank_candidates` (use shared ranking or `decay_score_with_usage_at`) |
| `crates/ctxgraph-core/tests/core_tests.rs` | All 19 decay test calls updated for new signatures + new sigmoid/recall-aware tests |
