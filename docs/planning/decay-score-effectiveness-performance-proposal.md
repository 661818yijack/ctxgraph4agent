# MemoryType::decay_score Effectiveness and Performance Proposal

## Context

`MemoryType::decay_score` currently computes freshness from:

- `base_confidence`
- `created_at`
- optional `ttl`
- the memory type's hard-coded curve
- `Utc::now()` inside the function

It is used by retrieval ranking through `score_candidate`, stale memory listing through `get_stale_memories`, and cleanup/statistics logic around decayed memories.

The current implementation is simple and cheap per call, but it leaves two important improvements on the table:

- Effectiveness: the model ignores `usage_count` and `last_recalled_at`, even though entities and edges already store both.
- Performance and correctness: batch call sites repeatedly compute `Utc::now()`, and stale-memory listing fetches recent rows first before filtering by decay score in Rust.

## Recommendation

Keep the current public behavior for compatibility, but add a deterministic, batch-friendly, recall-aware path.

### 1. Add a time-injected decay function

Add:

```rust
pub fn decay_score_at(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
    now: DateTime<Utc>,
) -> f64
```

Then make the existing method delegate:

```rust
pub fn decay_score(
    &self,
    base_confidence: f64,
    created_at: DateTime<Utc>,
    ttl: Option<Duration>,
) -> f64 {
    self.decay_score_at(base_confidence, created_at, ttl, Utc::now())
}
```

Benefits:

- Tests become deterministic because callers can pass a fixed `now`.
- Retrieval and stale-memory loops can compute `now` once per batch.
- Existing call sites do not need to change immediately.

### 2. Add a recall-aware scoring variant

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

Use `last_recalled_at` and `usage_count` as bounded memory reinforcement signals. The goal is not to make old memories immortal; it is to slow decay for memories that are repeatedly useful.

One conservative formula:

```rust
let anchor = last_recalled_at.unwrap_or(created_at);
let age_created = seconds_between(now, created_at);
let age_recalled = seconds_between(now, anchor);

let recall_boost = (1.0 + 0.15 * (usage_count as f64).ln_1p()).min(1.75);
let effective_ttl = ttl_secs * recall_boost;

let age = age_created.min(age_recalled + ttl_secs * 0.25);
```

Then apply the existing per-type curve using `effective_ttl`:

- `Fact` and `Decision`: exponential, half-life = `effective_ttl * 0.5`
- `Preference`: exponential, half-life = `effective_ttl * 0.7`
- `Experience`: linear over `effective_ttl`
- `Pattern`: unchanged, returns `base_confidence`

Rationale:

- Recency matters because a memory recalled yesterday should usually outrank an otherwise identical memory unused for months.
- Frequency matters because repeated recall is a useful proxy for durable relevance.
- The boost is capped so frequently recalled memories still age out unless renewed or converted into patterns/skills.

### 3. Use the recall-aware path in retrieval ranking

`RetrievalCandidate` already carries `usage_count`, but not `last_recalled_at`. Add:

```rust
pub last_recalled_at: Option<DateTime<Utc>>,
```

Then update candidate retrieval in SQLite to select `last_recalled_at` for entities and edges, and update `score_candidate` to call the recall-aware variant.

This makes the existing `touch_entity` and `touch_edge` methods materially affect future ranking, rather than only preserving metadata for cleanup.

### 4. Push stale-memory filtering down into SQL

`get_stale_memories` currently queries rows ordered by `created_at DESC` or `recorded_at DESC`, applies `LIMIT/OFFSET`, and only then filters by `decay_score < threshold` in Rust. That has two problems:

- It can miss older stale memories when recent fresh rows fill the limit.
- It does unnecessary Rust-side decay work.

Improve it by computing approximate threshold cutoffs per memory type and filtering in SQL before applying `LIMIT/OFFSET`.

For expired-only filtering, the existing stats logic already uses timestamp cutoffs. For threshold filtering, compute age cutoffs from the inverse of each curve:

- Exponential: `age = -ln(threshold / base_confidence) * half_life`
- Linear: `age = ttl * (1.0 - threshold / base_confidence)`

Clamp cutoff ages into `[0, ttl]`, and keep the Rust `decay_score_at` check as a final exact filter.

Use `unixepoch()` or precomputed RFC3339 cutoff timestamps rather than repeatedly calling `strftime('%s', ...)` where possible. SQLite documents `unixepoch()` as a direct numeric date/time function, avoiding equivalent `strftime` conversion overhead.

Reference: https://www.sqlite.org/lang_datefunc.html

## Research Notes

ACT-R-style declarative memory models treat memory availability as a function of traces, recency, and frequency. That supports using `last_recalled_at` and `usage_count` as bounded reinforcement signals in ctxgraph.

Reference: https://link.springer.com/article/10.1007/s42113-023-00189-y

FSRS-style spaced repetition models also separate retrievability from stability and use recall history to update future forgetting behavior. That model is likely too heavy for ctxgraph right now because ctxgraph does not store review outcomes or difficulty/stability parameters, but it supports the direction of making decay recall-aware.

Reference: https://github.com/open-spaced-repetition/fsrs4anki/wiki/The-Algorithm

## Suggested Implementation Order

1. Add `decay_score_at` and route existing `decay_score` through it.
2. Update tests to use fixed `now` values where assertions are precise.
3. Add `last_recalled_at` to `RetrievalCandidate` and populate it from entity/edge candidate queries.
4. Add `decay_score_with_usage_at` and update `score_candidate`.
5. Add focused tests:
   - recently recalled old fact outranks otherwise identical unrecalled old fact
   - usage boost is capped
   - expired unrecalled memory still scores `0.0`
   - pattern behavior remains unchanged
6. Push stale-memory threshold filtering into SQL and keep Rust decay as the exact final check.

## Risk and Compatibility

The safest rollout is to preserve the existing `decay_score` signature and behavior, then introduce recall-aware behavior only in `score_candidate`.

Cleanup should remain conservative at first. Do not make cleanup rely solely on recall-aware effective TTL until the ranking behavior has been validated, because cleanup has higher blast radius than ranking. For cleanup, keep hard TTL plus grace-period behavior unless a separate retention-policy change is explicitly desired.
