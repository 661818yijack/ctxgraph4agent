# Enhance `MemoryType::decay_score` 02

## Status

Proposed consolidated plan.

This document merges and reconciles:

- `docs/planning/enhance-decay-score.md`
- `docs/planning/decay-score-effectiveness-performance-proposal.md`
- `docs/planning/enhancement-decay-score.md`
- `docs/adr/decay-score-enhancement.md`

## Decision

Implement the improvement in two safe phases:

1. Make decay scoring deterministic and recall-aware for retrieval ranking.
2. Improve stale-memory SQL filtering and ranking code reuse.

Do not immediately replace the existing decay curves with power-law or sigmoid curves. Those are plausible future improvements, but they change ranking and cleanup semantics enough that they should be benchmarked separately against real retrieval behavior.

## Current Problems

`MemoryType::decay_score` currently:

- Calls `Utc::now()` internally, which makes exact testing awkward and causes tiny batch-to-batch clock skew.
- Ignores `usage_count` and `last_recalled_at`, even though `Entity` and `Edge` already store both.
- Uses a hard TTL cliff: exponential types can score about `0.25` at the TTL boundary, then `0.0` immediately after.
- Gives `Fact` and `Decision` the same curve, even though decisions often behave more like commitments than ordinary facts.
- Is called inside row loops in stale-memory and retrieval flows.

The highest-value fix is not a wholesale curve replacement. The highest-value fix is to use the signals the system already records, while preserving existing cleanup behavior until retention policy is deliberately changed.

## Chosen Design

### 1. Preserve the Existing Public Method

Keep:

```rust
pub fn decay_score(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
) -> f64
```

Make it delegate to a new deterministic variant:

```rust
pub fn decay_score_at(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,
) -> f64
```

This avoids a broad breaking change and lets tests and batch callers pass one stable `now`.

### 2. Add a Recall-Aware Variant for Ranking

Add:

```rust
pub fn decay_score_with_usage_at(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    last_recalled_at: Option<DateTime<Utc>>,
    usage_count: u32,
    now: DateTime<Utc>,
) -> f64
```

This should be used by `score_candidate`, not by cleanup initially.

Rationale:

- Ranking should reward memories that are repeatedly useful.
- Cleanup is destructive or archival, so it should keep hard TTL plus grace-period behavior until a separate retention-policy decision is made.

### 3. Use Bounded Recall Reinforcement

Use recall as a bounded freshness signal, not a full TTL reset.

Recommended formula:

```rust
let anchor = last_recalled_at
    .filter(|recalled| *recalled > created_at && *recalled <= now)
    .unwrap_or(created_at);

let age_created = seconds_between(now, created_at);
let age_recalled = seconds_between(now, anchor);

let recall_boost = (1.0 + 0.15 * (usage_count as f64).ln_1p()).min(1.75);
let effective_ttl = ttl_secs * recall_boost;

let effective_age = age_created.min(age_recalled + ttl_secs * 0.25);
```

Then apply the current per-type curve against `effective_age` and `effective_ttl`:

- `Fact`: exponential, half-life = `effective_ttl * 0.5`
- `Decision`: unchanged in phase 1, exponential, half-life = `effective_ttl * 0.5`
- `Preference`: exponential, half-life = `effective_ttl * 0.7`
- `Experience`: linear over `effective_ttl`
- `Pattern`: unchanged, returns `base_confidence`

Why this is better than a raw `last_recalled_at` reset:

- A memory recalled once yesterday does not become equivalent to a newly created memory.
- Repeated recall matters, but the boost is capped.
- Long-lived, useful memories are promoted in ranking without becoming immortal.

### 4. Add `last_recalled_at` to Retrieval Candidates

`RetrievalCandidate` already has `usage_count`. Add:

```rust
pub last_recalled_at: Option<DateTime<Utc>>,
```

Then update SQLite candidate retrieval for entities and edges to select and populate `last_recalled_at`.

This makes existing methods like `touch_entity` and `touch_edge` affect future ranking through both `usage_count` and recall recency.

### 5. Keep `base_confidence`

Do not remove `base_confidence` in this pass.

Some source docs argue it is effectively dead because several current call sites pass `1.0`. That is true for stale-memory flows, but `RetrievalCandidate` already carries `base_confidence`, and extraction or source-quality signals can use it later. Removing it now creates API churn without a meaningful runtime win.

### 6. Improve Stale-Memory Filtering

`get_stale_memories` should not fetch recent rows, apply `LIMIT/OFFSET`, and only then filter by decay score in Rust. That can miss older stale rows when fresh rows fill the page.

Change it to prefilter by age in SQL before pagination, then keep Rust decay as the exact final check.

For threshold filtering:

- Exponential approximate cutoff: `age = -ln(threshold / base_confidence) * half_life`
- Linear approximate cutoff: `age = ttl * (1.0 - threshold / base_confidence)`

Clamp cutoff ages to `[0, ttl]`.

Use precomputed cutoff timestamps or SQLite numeric date functions rather than repeated `strftime('%s', ...)` expressions where practical.

### 7. Deduplicate Ranking Logic

`Graph::rank_candidates` and `Storage::retrieve_for_context` currently duplicate the score, filter, and sort pipeline.

Extract a helper in `types.rs`:

```rust
pub fn rank_scored_candidates_at(
    candidates: Vec<RetrievalCandidate>,
    now: DateTime<Utc>,
) -> Vec<ScoredCandidate>
```

Then have both call sites reuse it. This keeps ranking behavior consistent after the recall-aware changes.

## Explicit Non-Goals for Phase 1

Do not remove the hard TTL cliff from cleanup yet.

Do not switch to power-law decay yet. The cognitive-science argument is reasonable, but the effect on ctxgraph retrieval quality should be measured before changing all curve behavior.

Do not switch `Decision` to sigmoid decay yet. A plateau curve may be semantically better for architectural decisions, but it should be introduced as a separate change with targeted tests and examples.

Do not make `last_recalled_at` an unlimited TTL reset. That would make accidental or noisy recall too powerful.

## Implementation Order

1. Add `decay_score_at` and make existing `decay_score` delegate to it.
2. Add tests for deterministic fixed-`now` behavior.
3. Add `last_recalled_at` to `RetrievalCandidate`.
4. Populate `last_recalled_at` in entity and edge candidate retrieval SQL.
5. Add `decay_score_with_usage_at`.
6. Update `score_candidate` to use recall-aware decay.
7. Extract shared ranking helper and update `Graph::rank_candidates` and `Storage::retrieve_for_context`.
8. Add SQL-side stale-memory prefiltering, keeping Rust decay as the exact final check.
9. Add tests for recall-aware ranking and stale pagination.

## Test Plan

Add or update tests for:

- `decay_score_at` returns the same value for repeated calls with fixed `now`.
- Existing `decay_score` behavior remains compatible.
- A recently recalled old `Fact` outranks an otherwise identical unrecalled old `Fact`.
- Recall boost is capped.
- Expired unrecalled memories still score `0.0`.
- `Pattern` behavior remains unchanged.
- `get_stale_memories` does not miss stale rows just because recent fresh rows fill the first page.
- `Graph::rank_candidates` and `Storage::retrieve_for_context` produce consistent ordering through the shared helper.

## Files Affected

- `crates/ctxgraph-core/src/types.rs`
  - `MemoryType::decay_score`
  - new `MemoryType::decay_score_at`
  - new `MemoryType::decay_score_with_usage_at`
  - `RetrievalCandidate`
  - `score_candidate`
  - new ranking helper
- `crates/ctxgraph-core/src/storage/sqlite.rs`
  - candidate retrieval SQL
  - `retrieve_for_context`
  - `get_stale_memories`
- `crates/ctxgraph-core/src/graph.rs`
  - `rank_candidates`
- `crates/ctxgraph-core/tests/core_tests.rs`
  - fixed-`now` decay tests
  - recall-aware ranking tests
- Inline tests in `crates/ctxgraph-core/src/types.rs`
  - update `RetrievalCandidate` construction helpers for `last_recalled_at`

## Future Work

After phase 1, benchmark alternative curves:

- Power-law decay for `Fact` and `Preference`
- Sigmoid or plateau decay for `Decision`
- Soft expiration tail for exponential types

Evaluate them against retrieval relevance, stale-memory surfacing, and cleanup behavior before adopting them globally.

## Why This Is the Best Combined Plan

It keeps the strongest low-risk ideas from all four source docs:

- Deterministic `now` injection.
- Recall-aware scoring using existing fields.
- SQL-side stale filtering.
- Ranking logic deduplication.

It avoids the riskiest immediate changes:

- Breaking all call sites unnecessarily.
- Removing `base_confidence` prematurely.
- Changing cleanup semantics as a side effect of ranking work.
- Replacing the decay curve without benchmark evidence.
