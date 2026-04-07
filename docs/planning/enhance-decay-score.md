# Enhancement Proposal: `MemoryType::decay_score`

**File:** `crates/ctxgraph-core/src/types.rs`  
**Function:** `MemoryType::decay_score` (line 88)  
**Date:** 2026-04-07

---

## Current Signature

```rust
pub fn decay_score(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
) -> f64
```

---

## Issues Found

### Issue 1: Impure function — `Utc::now()` called internally

```rust
// line 111
let age_secs = (Utc::now() - created_at).num_seconds().max(0) as f64;
```

Two concrete problems:

- **Untestable against exact timestamps** — every test must race the clock or use fuzzy tolerances (`< 1e-6`)
- **Batch clock skew** — in `sqlite.rs` lines 1287 and 1345, `decay_score` is called inside a row-by-row loop. Each call gets a different `Utc::now()`. A 1000-row cleanup sweep spans milliseconds of drift across results

### Issue 2: Hard cliff at TTL boundary

```rust
if age_secs > ttl_secs {
    return 0.0;  // hard cliff
}
```

For exponential types (Fact, Decision, Preference), the curve never reaches 0 naturally. At the TTL boundary a Fact scores `exp(-2*ln2) ≈ 0.25`, then one second later jumps to `0.0` — a 25-point instant drop. `Experience` avoids this since linear decay already reaches 0 at TTL, but Fact/Decision/Preference don't.

### Issue 3: `last_recalled_at` is ignored

`Entity` and `Edge` both carry `last_recalled_at: Option<DateTime<Utc>>`. CLAUDE.md explicitly calls this out as the **implicit re-verification mechanism**: *"if a memory is recalled and used → auto-renew TTL"*. But `decay_score` only ages from `created_at`. A memory recalled yesterday ages the same as one never touched.

### Issue 4: Decision and Fact have identical curves

The tests literally assert:

```rust
assert!((fact_score - decision_score).abs() < 1e-10, "...should have identical decay");
```

But semantically they are different. A **Fact** is empirical data — appropriate to decay gradually as it might become outdated. A **Decision** is an architectural commitment — it stays fully valid until explicitly overridden or approaching review time, then drops sharply. Exponential decay is the wrong shape for Decisions. They need a plateau: stable for most of TTL, sharp drop at the end.

---

## Proposed Improvements

### Improvement 1: Pure function — add `now` parameter

Change the signature to:

```rust
pub fn decay_score(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,                          // NEW
) -> f64
```

Callers compute `now` once per batch and pass it down:

```rust
// sqlite.rs — before the cleanup loop
let now = Utc::now();
for row in rows {
    let decay = memory_type.decay_score(1.0, created_at, ttl, now);
    ...
}
```

`score_candidate` also receives `now` from `rank_candidates`, which controls the batch. Zero semantic change. Fixes testability and eliminates batch clock skew.

**Call sites to update:**
- `crates/ctxgraph-core/src/types.rs:601` — `score_candidate`
- `crates/ctxgraph-core/src/storage/sqlite.rs:1287` — entity cleanup loop
- `crates/ctxgraph-core/src/storage/sqlite.rs:1345` — edge cleanup loop

---

### Improvement 2: Use `last_recalled_at` for implicit TTL reset

Add a second new parameter:

```rust
pub fn decay_score(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    last_recalled_at: Option<DateTime<Utc>>,     // NEW
    now: DateTime<Utc>,
) -> f64
```

Change the age computation from:

```rust
let age_secs = (now - created_at).num_seconds().max(0) as f64;
```

To:

```rust
let effective_origin = last_recalled_at
    .filter(|&r| r > created_at)
    .unwrap_or(created_at);
let age_secs = (now - effective_origin).num_seconds().max(0) as f64;
```

A memory recalled 5 days ago now ages from 5 days ago, regardless of when it was created. This directly implements the implicit re-verification design goal from CLAUDE.md. A frequently-used memory that is 60 days old but was recalled yesterday effectively has a 1-day age — it stays fresh.

**Example:**
- Memory created 60 days ago, 90d TTL, never recalled → ages 60 days, score ≈ 0.40
- Memory created 60 days ago, 90d TTL, recalled 5 days ago → ages 5 days, score ≈ 0.93

---

### Improvement 3: Soft expiry — remove the cliff for exponential types

Replace the hard drop with a smooth ramp-down in the last 10% of TTL:

```rust
// Before — hard cliff:
if age_secs > ttl_secs {
    return 0.0;
}
// ... exponential computed normally

// After — soft expiry zone:
let soft_start = ttl_secs * 0.9;

if age_secs > ttl_secs {
    return 0.0;
}

if age_secs >= soft_start {
    // Compute score at the start of the soft zone
    let ttl_score = base_confidence * decay_exponential(soft_start, half_life);
    // Linear ramp from ttl_score → 0.0 over the last 10% of TTL
    let ramp = 1.0 - (age_secs - soft_start) / (ttl_secs * 0.1);
    return (ttl_score * ramp.max(0.0)).max(0.0);
}

// Normal exponential below soft_start
base_confidence * decay_exponential(age_secs, half_life)
```

`Experience` is unaffected since linear decay already reaches 0 at TTL. For Fact with a 90-day TTL: instead of jumping from ~0.26 to 0.0 at day 90, it ramps from ~0.28 at day 81 down smoothly to 0.0 at day 90.

---

### Improvement 4: Sigmoid decay for Decision

Decisions are architectural commitments. They need a **plateau curve**: stay near full confidence for most of the TTL (the "still valid" period), then drop sharply near the end (the "needs review" signal).

Add a new private function:

```rust
/// Sigmoid decay: stays near 1.0 for most of the lifetime,
/// then drops sharply after the inflection point.
///
/// f(t) = 1 / (1 + exp(k * (t - inflection)))
/// where t = age / ttl, k = steepness, inflection = 0.8 (drops after 80% of TTL)
///
/// Returns 1.0 at age=0, ~0.5 at age=0.8*ttl, ~0.0 at age=ttl
fn decay_sigmoid(age_secs: f64, ttl_secs: f64) -> f64 {
    let k = 10.0;
    let inflection = 0.8;
    let t = age_secs / ttl_secs;
    1.0 / (1.0 + (k * (t - inflection)).exp())
}
```

Change the match arm from:

```rust
// Before — Decision and Fact share the same curve:
MemoryType::Fact | MemoryType::Decision => {
    let half_life = ttl_secs * 0.5;
    base_confidence * decay_exponential(age_secs, half_life)
}
```

To:

```rust
// After — differentiated curves:
MemoryType::Fact => {
    let half_life = ttl_secs * 0.5;
    base_confidence * decay_exponential(age_secs, half_life)
}
MemoryType::Decision => {
    base_confidence * decay_sigmoid(age_secs, ttl_secs)
}
```

**Score profile for a 90-day Decision TTL:**

| Age | Score |
|-----|-------|
| Day 0  | ~1.00 |
| Day 10 | ~0.98 |
| Day 50 | ~0.88 |
| Day 72 | ~0.50 (inflection) |
| Day 82 | ~0.12 |
| Day 90 | ~0.02 |

Compare to current exponential (half-life=45d): day 50 → 0.54, day 72 → 0.33. The sigmoid keeps the decision fully trusted for longer, then gives a clear "this needs review" signal.

**Note:** The existing test `test_decay_fact_and_decision_identical` must be removed and replaced with separate tests for each curve.

---

## Final Proposed Signature

```rust
pub fn decay_score(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    last_recalled_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> f64
```

---

## Implementation Priority

| # | Improvement | Impact | Complexity | Breaks tests? |
|---|---|---|---|---|
| 1 | Pure function (pass `now`) | Fixes testability + batch skew | Low — add 1 param, update 3 call sites | Yes — all `decay_score` call sites |
| 2 | `last_recalled_at` implicit reset | Implements stated CLAUDE.md design goal | Low — add 1 param, 3-line logic change | Yes — callers must pass `None` or real value |
| 3 | Soft expiry ramp | Removes jarring cliff for exponential types | Medium — soft zone logic per type | Minimal — boundary behavior changes |
| 4 | Sigmoid for Decision | Semantically correct curve | Low — 1 new function, split match arm | Yes — `test_decay_fact_and_decision_identical` |

**Ship order:** Improvements 1 + 2 together (one breaking change, two parameters added at once). Then 4. Then 3 last (most test churn).

---

## Notes

- Improvements 1 and 2 should be shipped together to minimize the number of breaking signature changes.
- The `RetrievalCandidate` struct already carries `usage_count` but not `last_recalled_at`. To fully implement Improvement 2, `last_recalled_at: Option<DateTime<Utc>>` must also be added to `RetrievalCandidate` and populated from SQLite in the candidate retrieval query.
- `enforce_budget` is unaffected by all of these changes — it operates on already-scored candidates.
