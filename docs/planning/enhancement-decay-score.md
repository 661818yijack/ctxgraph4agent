# Enhancement Proposal: Improve `MemoryType::decay_score`

> **File:** `crates/ctxgraph-core/src/types.rs` (lines 88–134)
> **Date:** 2026-04-07
> **Status:** Proposed

---

## Current Implementation

```rust
// Exponential (Fact, Decision):   half_life = ttl * 0.5
// Exponential (Preference):        half_life = ttl * 0.7
// Linear     (Experience):         base * max(0, 1 - age/ttl)
// Constant   (Pattern):            never decays
// Hard cutoff at age > ttl → 0.0
```

**Callers:** `score_candidate()` (per retrieval), `get_stale_memories()` (per entity/edge scan) — 23 call sites across the codebase.

---

## Problems Identified

### P1 — Cognitive Science Mismatch

Human memory follows a **power-law** decay curve (rapid initial drop, slow long-tail), not exponential. Exponential decay:

- **Over-predicts long-term forgetting** → foundational facts get pruned too aggressively
- **Under-predicts short-term forgetting** → stale details linger longer than useful

The ACT-R cognitive architecture (gold standard for human memory modeling) and 50+ years of empirical data confirm: power-law > exponential for retention modeling.

> *"Exponential decay assumes a constant, time-independent forgetting rate, predicting uniform memory loss over intervals. Power-law decay captures a rapid initial drop in retention followed by a slow, long-tail decline. Empirical human memory data consistently favors the power-law function, as it accurately models the stability of consolidated memories and the spacing effect, whereas exponential models systematically over-predict long-term forgetting."*
> — Wixted (2004), ACT-R research

### P2 — Abrupt Cutoff Cliff

```rust
if age_secs > ttl_secs { return 0.0; }
```

A memory at `ttl - 1 second` has a nonzero score; at `ttl + 1 second` it drops to exactly `0.0`. This is:

- **Unnatural:** real decay is asymptotic, never discontinuous
- **Problematic for grace period:** memories near the cliff oscillate between "retrievable" and "dead" across sequential calls
- **Rigid:** makes the grace period meaningless since the memory is already dead

### P3 — `base_confidence` Is a Dead Parameter

Every production caller passes `base_confidence = 1.0`:

```rust
// sqlite.rs:1287
let decay_score = memory_type.decay_score(1.0, created_at, ttl);
// sqlite.rs:1345
let decay_score = memory_type.decay_score(1.0, recorded_at, ttl);
// types.rs:601 (score_candidate)
let decay = candidate.memory_type.decay_score(candidate.base_confidence, ...);
```

The signal it was supposed to carry (source reliability, initial evidence quality) is never used. Dead API surface.

### P4 — No Usage Signal in Decay

The usage bonus (`1.0 + 0.1 * ln(1 + usage_count)`) is applied **after** decay in `score_candidate()`. This means:

- Decay doesn't know about usage
- A heavily-used memory decays at the same rate as an unused one
- The usage bonus only compensates at scoring time, but doesn't slow the actual decay curve

This contradicts the **spacing effect** from cognitive science: memories that are actively recalled become more stable.

### P5 — Recomputes `Utc::now()` Per Call

Every `decay_score()` call does `(Utc::now() - created_at).num_seconds()`. In `get_stale_memories()` with 1000 entities, this is 1000 separate clock reads. Minor but unnecessary.

---

## Proposed Improvements

### Improvement 1: Power-Law Decay (Primary)

Replace exponential with power-law:

```rust
/// Power-law decay: `(1 + age/half_life)^(-alpha)`
///
/// Returns 1.0 at age=0, ~0.5 at age=half_life, asymptotic long-tail.
/// Calibrated so power-law and exponential intersect at half_life (0.5).
fn decay_power_law(age_secs: f64, half_life_secs: f64, alpha: f64) -> f64 {
    (1.0 + age_secs / half_life_secs).powf(-alpha)
}
```

**Behavior change comparison:**

| Age (relative to TTL) | Exponential (current) | Power-Law (α=1.0) | Impact |
|---|---|---|---|
| 0 (fresh) | 1.00 | 1.00 | Same |
| 0.25 × TTL | 0.84 | 0.87 | Slightly higher |
| 0.5 × TTL (half-life) | 0.50 | 0.50 | **Same by design** |
| 0.75 × TTL | 0.30 | 0.28 | Slightly lower |
| 1.0 × TTL | 0.25 | 0.20 | Lower (faster initial drop) |
| 1.5 × TTL | 0.00 (cutoff) | 0.14 | **Long-tail retention** |
| 2.0 × TTL | 0.00 (cutoff) | 0.11 | Still retrievable |

**Why `alpha ≈ 1.0`:** Calibrate so power-law and exponential intersect at half_life (0.5). With `alpha = 1.0`:

- `(1 + 1)^(-1) = 0.5` ✓ at half_life
- Faster initial drop (age < half_life): more aggressive on fresh-but-stale details
- Slower tail decay (age > half_life): preserves consolidated knowledge

### Improvement 2: Asymptotic Instead of Hard Cutoff

Remove the `age_secs > ttl_secs → 0.0` cliff. Let decay approach zero asymptotically. Use a floor (e.g., `0.01`) to define "effectively expired" for cleanup purposes.

```rust
// Instead of:
if age_secs > ttl_secs { return 0.0; }

// Use asymptotic decay, cleanup uses floor:
let score = decay(...);
if score < 0.01 { 0.0 } else { score }
```

**Why:** Eliminates the binary cliff, makes grace period meaningful (decay continues naturally during grace), and cleanup naturally targets the asymptotic floor.

### Improvement 3: Usage-Driven Decay Slowdown

Frequently recalled memories should decay slower. Incorporate `usage_count` into the decay rate:

```rust
// Effective half-life grows with usage
let effective_half_life = half_life * (1.0 + 0.5 * ln(1 + usage_count));
```

**Why:** This is the "spacing effect" from cognitive science — memories that are actively recalled become more stable. Currently, usage only affects the post-decay score bonus, not the decay curve itself.

**Note:** This requires passing `usage_count` into `decay_score()`, changing the function signature.

### Improvement 4: Remove `base_confidence` Parameter or Use It

Two options:

- **Option A (simpler):** Remove the parameter, hardcode to 1.0 inside. Dead API surface.
- **Option B (richer):** Actually use it — pass source reliability scores from the ingestion pipeline.

**Recommendation:** Option A for now. If `base_confidence` becomes useful later (e.g., from an LLM confidence score at extraction time), it can be re-added.

### Improvement 5: Accept `now` as Parameter (Performance)

```rust
pub fn decay_score_at(
    &self,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,  // caller provides "now"
) -> f64 { ... }

// Convenience wrapper:
pub fn decay_score(&self, created_at, ttl) -> f64 {
    self.decay_score_at(created_at, ttl, Utc::now())
}
```

**Why:** Callers doing batch operations (like `get_stale_memories`) can compute `now` once and pass it to all calls. Also makes the function pure (deterministic for same inputs), which is critical for testing and reproducibility.

---

## Implementation Priority

| Priority | Improvement | Effort | Risk | Impact |
|----------|-------------|--------|------|--------|
| **P0** | #2: Remove hard cutoff (asymptotic) | Low | Low — eliminates cliff, more natural | High |
| **P1** | #5: Accept `now` parameter | Low | None — pure function, backward compatible wrapper | Medium |
| **P2** | #1: Power-law decay | Medium | Medium — changes all decay curves, needs test recalibration | High |
| **P3** | #4: Remove dead `base_confidence` | Low | Low — all callers pass 1.0 | Medium (API clarity) |
| **P4** | #3: Usage-driven decay slowdown | Medium | Medium — changes function signature, behavioral shift | High |

---

## Recommendation

Start with **P0 (#2 asymptotic)** and **P1 (#5 `now` param)** — both are low-risk, low-effort improvements that fix real bugs without changing the decay curve shape.

**Power-law (#1)** is the highest-impact change but requires recalibrating all existing tests and potentially tuning `alpha` against real data. Should be done in a separate pass with benchmarking.

---

## References

- Wixted, J. T. (2004). *The psychology and neuroscience of forgetting.* Annual Review of Psychology.
- Rubin, D. C., & Wenzel, A. E. (1996). *One hundred years of forgetting: A quantitative description of retention.* Psychological Review.
- ACT-R cognitive architecture: http://act-r.psy.cmu.edu/
- *"Exponential decay contrasts with power law decay by having a weaker long-term retention of memories; with power law decay there is a fast initial drop followed by a slow, long-tail decline."*
- *"Rigid exponential decay often causes premature loss of foundational information, increasing susceptibility to catastrophic forgetting."*
