# B3: Compression Triggers (time-based, size-based)

## Goal

Add an orchestration layer that decides *which* episodes to compress and *when*, then calls the existing `compress_episodes` per-group logic (B1: LLM summary, B2: edge inheritance already done).

## Current Context

- `Graph::compress_episodes(&[episode_ids])` — takes explicit IDs, generates LLM summary, delegates to Storage
- `Storage::compress_episodes(&[episode_ids], summary)` — handles per-group compression with edge inheritance (B2 done)
- `Storage::list_uncompressed_episodes(before)` — finds old uncompressed episodes (manual use only)
- `Storage::get_compression_groups(before)` — groups episodes temporally (used by pattern extraction, NOT compression)
- No automatic trigger mechanism exists

## What's Missing

| Missing | Description |
|---------|-------------|
| Time-based trigger | Find episodes older than N days, compress in temporal batches |
| Size-based trigger | If uncompressed count > threshold, compress oldest first |
| Batch orchestration | `run_batch_compression` that iterates groups, calls LLM per group |

## Implementation Plan

### Step 1: Add `CompressionConfig` struct

File: `crates/ctxgraph-core/src/types.rs`

```rust
pub struct CompressionConfig {
    /// Max age (days) of episodes to consider for compression
    pub max_age_days: u32,
    /// Max episodes per compression group
    pub batch_size: usize,
    /// For size-based: minimum uncompressed count before triggering
    pub size_threshold: Option<usize>,
}
```

### Step 2: Add `CompressionResult` type

File: `crates/ctxgraph-core/src/types.rs`

```rust
pub struct CompressionResult {
    pub groups_compressed: usize,
    pub episodes_compressed: usize,
    pub skipped_already_compressed: usize,
    pub errors: Vec<String>,
}
```

### Step 3: Add `Storage::run_batch_compression` (RED test first)

File: `crates/ctxgraph-core/src/storage/sqlite.rs`

- Takes `CompressionConfig`, finds uncompressed episodes older than `max_age_days`
- Groups by temporal window (by day, up to `batch_size` per group)
- For each group: generate summary (placeholder — LLM call later), call `compress_episodes`
- Returns `CompressionResult`

**Test first** → `test_run_batch_compression_time_based`:
- Create 5 episodes across 3 different days
- Call `run_batch_compression(config with max_age_days=7, batch_size=2)`
- Assert: 2 groups compressed (day1:2ep, day2:2ep, day3:1ep not enough to trigger second batch)
- Assert: correct `CompressionResult` counts

### Step 4: Add `Storage::run_compression_if_needed`

File: `crates/ctxgraph-core/src/storage/sqlite.rs`

- Count uncompressed episodes
- If count >= size_threshold, call `run_batch_compression`
- If not, return zeroed `CompressionResult` with `skipped_already_compressed = count`

**Test first** → `test_run_compression_if_needed_skips_when_under_threshold`:
- Create 3 uncompressed episodes (threshold = 5)
- Call `run_compression_if_needed(threshold=5, batch_size=10)`
- Assert: 0 groups compressed, 3 skipped

**Test** → `test_run_compression_if_needed_triggers_when_at_threshold`:
- Create 6 uncompressed episodes (threshold = 5)
- Call `run_compression_if_needed(threshold=5, batch_size=10)`
- Assert: episodes compressed

### Step 5: Add `Graph::run_batch_compression` and `Graph::run_compression_if_needed`

File: `crates/ctxgraph-core/src/graph.rs`

- Wrap Storage methods with config + error handling
- `run_batch_compression(config)` — time-based
- `run_compression_if_needed(threshold, batch_size)` — size-based

### Step 6: Add tests for batch grouping behavior

**Test** → `test_batch_compression_groups_by_temporal_window`:
- Create 10 episodes: 4 from day1, 4 from day2, 2 from day3
- batch_size=3
- Assert: day1 → 1 group (3ep), day2 → 1 group (3ep), day3 → 1 group (2ep)
- Total: 3 groups compressed

**Test** → `test_batch_compression_idempotent_on_already_compressed`:
- Compress a group, then call again with same episodes
- Assert: second call skips already-compressed, returns correct counts

## Files to Change

| File | Change |
|------|--------|
| `crates/ctxgraph-core/src/types.rs` | Add `CompressionConfig`, `CompressionResult` |
| `crates/ctxgraph-core/src/storage/sqlite.rs` | Add `run_batch_compression`, `run_compression_if_needed`, `get_compression_groups_for_compression` |
| `crates/ctxgraph-core/src/graph.rs` | Add public wrappers `run_batch_compression`, `run_compression_if_needed` |
| `crates/ctxgraph-core/tests/core_tests.rs` | Add B3 tests |

## Tests to Write (TDD order)

1. `test_run_batch_compression_time_based`
2. `test_run_compression_if_needed_skips_when_under_threshold`
3. `test_run_compression_if_needed_triggers_when_at_threshold`
4. `test_batch_compression_groups_by_temporal_window`
5. `test_batch_compression_idempotent_on_already_compressed`

## Verification

```bash
cargo test -p ctxgraph test_run_batch 2>&1 | grep -E "(test |passed|failed|FAILED)"
cargo test -p ctxgraph test_run_compression_if_needed 2>&1 | grep -E "(test |passed|failed|FAILED)"
cargo test -p ctxgraph 2>&1 | tail -5  # full suite, no regressions
```

## Open Questions

1. **LLM summary placeholder**: For now each group gets `"Group of N episodes from [date range]"` as summary. When LLM integration is ready, swap this out.
2. **Grouping strategy**: Day-based grouping seems natural but should we do week-based? Configurable window size?
3. **Error handling in batch**: If one group fails, should we continue with others? (Yes — partial success is useful)