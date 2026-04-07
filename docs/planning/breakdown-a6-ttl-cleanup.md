# Must Have Features - A6 TTL Cleanup Breakdown

A6: TTL enforcement and cleanup — lazy background sweep for expired memories.

---

## Feature 1: cleanup_expired Function

### L1: Storage
### L2: TTL Cleanup
### L3: cleanup_expired
### L4: `fn cleanup_expired(&self, grace_period_days: i64) -> Result<CleanupResult>`
  - Query all entities/edges where `decay_score = 0.0` AND `age > grace_period_days`
  - Delete Fact and Experience nodes permanently
  - Archive Preference and Decision nodes (set `archived_at` timestamp, keep in DB)
  - Never delete Pattern nodes (they never expire)
  - Return CleanupResult with counts per type

### L5: Spec
```rust
// Input: grace_period_days = 7 (7-day grace period after decay hits 0)
//
// Output: CleanupResult {
//     entities_deleted: 23,
//     entities_archived: 5,
//     edges_deleted: 87,
//     patterns_preserved: 12,  // always 0 deletions
//     duration_ms: 45,
// }
//
// Behavior:
// - Facts with decay_score=0 AND age > grace_period_days → DELETE
// - Experiences with decay_score=0 AND age > grace_period_days → DELETE
// - Preferences with decay_score=0 AND age > grace_period_days → ARCHIVE (not delete)
// - Decisions with decay_score=0 AND age > grace_period_days → ARCHIVE (not delete)
// - Patterns → NEVER touched
// - Nodes with decay_score > 0 → preserved (still have useful freshness)
// - Nodes within grace_period → preserved (give them a final chance to be re-verified)
```

---

## Feature 2: Lazy Trigger Mechanism

### L1: Storage
### L2: TTL Cleanup
### L3: Lazy Trigger
### L4: `fn maybe_trigger_cleanup(&self) -> Result<Option<CleanupResult>>`
  - Check `system_metadata` table for `last_cleanup_at` and `query_count_since_cleanup`
  - If `query_count_since_cleanup >= cleanup_interval` (default 100):
    - Set `cleanup_in_progress = true`
    - Run `cleanup_expired()` with grace_period_days
    - Reset `query_count_since_cleanup = 0`
    - Update `last_cleanup_at`
    - Set `cleanup_in_progress = false`
  - Return `None` if not triggered, `Some(CleanupResult)` if ran
  - If another cleanup is already in progress, skip silently

### L5: Spec
```rust
// Input: system_metadata has { "last_cleanup_at": "2026-04-01T00:00:00Z", "query_count_since_cleanup": 100, "cleanup_interval": 100 }
//
// After retrieve_for_context() call:
//   query_count_since_cleanup = 101 >= 100 → trigger cleanup
//   cleanup_in_progress was false → proceed
//
// Output: Some(CleanupResult { entities_deleted: 5, ... })
// system_metadata updated: last_cleanup_at = now, query_count_since_cleanup = 0
//
// If cleanup_in_progress = true: skip, return None
// If query_count_since_cleanup < cleanup_interval: skip, return None
```

---

## Feature 3: System Metadata Table

### L1: Storage
### L2: TTL Cleanup
### L3: System Metadata for Cleanup
### L4: Storage layer changes for cleanup tracking
  - Add `cleanup_interval` field (default 100 queries)
  - Add `last_cleanup_at` timestamp
  - Add `query_count_since_cleanup` counter
  - Add `cleanup_in_progress` boolean flag
  - Initialize on first run if not present

### L5: Spec
```rust
// system_metadata table schema:
// key: TEXT PRIMARY KEY
// value: TEXT (JSON encoded)
//
// Initial values after migration:
// { "cleanup_interval": 100, "last_cleanup_at": null, "query_count_since_cleanup": 0, "cleanup_in_progress": false }
//
// increment_query_count() called every retrieve_for_context()
// reset_query_count() called after successful cleanup
```

---

## Feature 4: Integration with retrieve_for_context

### L1: Core
### L2: Retrieval
### L3: Cleanup Trigger Integration
### L4: Call `maybe_trigger_cleanup()` inside `retrieve_for_context()`
  - After retrieval completes, call `maybe_trigger_cleanup()`
  - If cleanup runs, log result (don't fail the retrieval)
  - Cleanup failure should not block retrieval

### L5: Spec
```rust
// Inside retrieve_for_context():
// 1. Execute A4a (tiered retrieval)
// 2. Execute A4b (score + rank)
// 3. Execute A4c (enforce budget)
// 4. Call maybe_trigger_cleanup() → runs cleanup if interval reached
// 5. Return ranked memories to caller
//
// If step 4 fails: log warning, continue (cleanup failure ≠ retrieval failure)
```

---

## Feature 5: CleanupResult Struct

### L1: Types
### L2: TTL Cleanup
### L3: CleanupResult
### L4: `struct CleanupResult`
  - `entities_deleted: u32`
  - `entities_archived: u32`
  - `edges_deleted: u32`
  - `patterns_preserved: u32` (always 0, informational)
  - `duration_ms: u64`

### L5: Spec
```rust
// CleanupResult represents what the cleanup sweep did.
//
// patterns_preserved is always 0 for deletion count
// but the field exists to document that patterns were checked and intentionally preserved
```

---

## Feature 6: Grace Period Behavior

### L1: Storage
### L2: TTL Cleanup
### L3: Grace Period
### L4: Grace period ensures recently-expired nodes get a final chance
  - grace_period_days default = 7
  - A node with decay_score=0 but age < grace_period_days is NOT deleted yet
  - This gives the re-verification system (Phase C) a window to act before permanent deletion
  - Only after grace_period expires does deletion happen

### L5: Spec
```rust
// Node with:
//   recorded_at = 180 days ago
//   ttl = 90 days
//   decay_score = 0.0 (fully decayed)
//   grace_period_days = 7
//
// On day 181: grace_period NOT expired → preserved (still in grace)
// On day 188: grace_period expired → DELETE (if Fact/Experience) or ARCHIVE (if Preference/Decision)
//
// This window allows C2 (active re-verification) to surface and potentially renew the memory
```

---

## Feature 7: CLI forget Command (Manual Expire)

### L1: CLI
### L2: Memory Management
### L3: forget
### L4: `ctxgraph forget <entity_id> [--hard]` or `ctxgraph forget --type experience`
  - Immediately mark a specific memory as expired (set decay_score=0)
  - `--hard` flag: delete immediately instead of waiting for cleanup
  - `--type` flag: expire all memories of a given type
  - Returns confirmation with what was expired

### L5: Spec
```rust
// ctxgraph forget ent_123
// Output: "Entity ent_123 marked for deletion. Will be removed in next cleanup sweep."
//
// ctxgraph forget ent_456 --hard
// Output: "Entity ent_456 permanently deleted."
//
// ctxgraph forget --type experience
// Output: "Marked 47 experiences for deletion. Will be removed in next cleanup sweep."
```

---

## Feature 8: stats MCP Tool (Cleanup Visibility)

### L1: MCP
### L2: Tools
### L3: stats
### L4: `stats` MCP tool showing memory health including cleanup state
  - `total_entities` by type
  - `decayed_entities` (decay_score=0) by type
  - `last_cleanup_at` timestamp
  - `queries_since_cleanup`
  - `next_cleanup_in` (queries until next scheduled cleanup)

### L5: Spec
```rust
// Input: stats tool call
// Output: {
//     "total_entities": {"fact": 150, "experience": 89, "pattern": 45, "preference": 12},
//     "decayed_entities": {"fact": 8, "experience": 23, "pattern": 0, "preference": 2},
//     "last_cleanup_at": "2026-04-05T10:00:00Z",
//     "queries_since_cleanup": 73,
//     "next_cleanup_in": 27,
//     "cleanup_in_progress": false
// }
```

---
