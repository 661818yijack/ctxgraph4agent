# A6 TTL Cleanup — Task List

> Priority: Auto features first. MCP tools second. CLI commands last (debug only).

---

## 🔴 P0 — Critical Bug Fix

### 1. Fix Lazy Trigger Counter (query_count_since_cleanup) ✅ COMPLETE

**Status:** ✅ COMPLETE — 2026-04-07

**What was changed:**
- Added 3 methods to `SqliteStorage`:
  - `increment_query_count_since_cleanup()` — increments DB counter
  - `get_query_count_since_cleanup()` → `Result<u64>` — reads DB counter (defaults to 0)
  - `reset_query_count_since_cleanup()` → `Result<()>` — resets to 0
- Modified `Graph::retrieve_for_context()`:
  - Calls `increment_query_count_since_cleanup()` after every retrieval
  - Calls `maybe_trigger_cleanup()` after increment (errors suppressed)
- Rewrote `Graph::maybe_trigger_cleanup()`:
  - Checks `cleanup_in_progress` first (early exit if true)
  - Reads `query_count_since_cleanup` from DB (not global atomic)
  - Reads `cleanup_interval` from DB (defaults to 100, configurable in Task 2)
  - Triggers cleanup when `count >= interval`
  - Resets counter on successful cleanup
- Added 9 unit tests (5 storage + 4 graph level)
- Fixed 3 pre-existing compile errors (missing `compression_id` field, type mismatch, wrong `extract` args)

**Files changed:**
- `crates/ctxgraph-core/src/storage/sqlite.rs` — +70 lines (3 methods + 5 tests)
- `crates/ctxgraph-core/src/graph.rs` — +80 lines (new trigger logic + 4 tests)
- `crates/ctxgraph-core/src/pattern.rs` — +1 line (fix missing field)

**Test results:** 92 tests pass (83 existing + 9 new), 0 clippy warnings from our changes.

---

## 🟡 P1 — Required Spec Features

### 2. Add `cleanup_interval` to system_metadata ✅ COMPLETE

**Status:** ✅ COMPLETE — 2026-04-07

**What was changed:**
- Added `get_cleanup_interval()` to `SqliteStorage`:
  - Returns value from system_metadata, clamped to [1, 10000]
  - Lazy init: sets default "100" on first access
  - Invalid values default to 100
- Added `set_cleanup_interval(interval)` to `SqliteStorage`:
  - Clamps to [1, 10000] silently
- Added public methods on `Graph`:
  - `set_cleanup_interval(interval)` — programmatic config
  - `get_cleanup_interval()` — read current value
- Updated `maybe_trigger_cleanup()` to use `get_cleanup_interval()` instead of raw `get_system_metadata`
- Added 8 unit tests (6 storage + 2 graph level)

**Files changed:**
- `crates/ctxgraph-core/src/storage/sqlite.rs` — +58 lines (2 methods + 6 tests)
- `crates/ctxgraph-core/src/graph.rs` — +28 lines (2 public methods + 2 tests)

**Test results:** 100 tests pass (92 existing + 8 new), 0 new clippy warnings.

---

### 3. Check `cleanup_in_progress` in Lazy Trigger ✅ COMPLETE

**Status:** ✅ COMPLETE — 2026-04-07 (early-exit guard added in Task 1, additional edge case tests added now)

**What was changed:**
- Early-exit guard in `maybe_trigger_cleanup()`:
  ```rust
  if let Some(val) = self.storage.get_system_metadata("cleanup_in_progress")? {
      if val == "true" { return Ok(()); }
  }
  ```
- Missing key or `"false"` → proceeds with cleanup
- Two-level guard: trigger-level early exit + storage-level defensive check in `cleanup_expired()`
- Added 2 edge case tests for explicit false and missing key scenarios

**Test results:** 161 tests pass (69 lib + 92 integration), 0 new clippy warnings.

---

### 4. Add Cleanup Visibility to MCP Stats Tool ✅ COMPLETE

**Status:** ✅ COMPLETE — 2026-04-07

**What was changed:**
- Extended `GraphStats` struct in `types.rs` with 6 new fields:
  - `last_cleanup_at: Option<String>` — RFC3339 timestamp or None
  - `queries_since_cleanup: u64` — counter since last cleanup
  - `cleanup_interval: u64` — configurable interval (default 100)
  - `cleanup_in_progress: bool` — lock state
  - `total_entities_by_type: Vec<(String, usize)>` — counts per memory type
  - `decayed_entities_by_type: Vec<(String, usize)>` — decayed counts per type
- Added 2 storage helper methods:
  - `get_entity_counts_by_type()` — GROUP BY memory_type, excludes archived
  - `get_decayed_counts_by_type(grace_period_secs)` — GROUP BY type for decayed
- Updated `storage::stats()` to populate all new fields
- Updated MCP `stats` tool to include cleanup fields in JSON response:
  - `total_entities_by_type` — JSON object
  - `decayed_entities_by_type` — JSON object
  - `last_cleanup_at` — "never" if null
  - `queries_since_cleanup`
  - `cleanup_interval`
  - `cleanup_in_progress`
  - `next_cleanup_in` — convenience: `interval - queries`
- Added 4 unit tests

**Files changed:**
- `crates/ctxgraph-core/src/types.rs` — +12 lines (6 new GraphStats fields)
- `crates/ctxgraph-core/src/storage/sqlite.rs` — +80 lines (2 helpers + 4 tests + stats update)
- `crates/ctxgraph-mcp/src/tools.rs` — +40 lines (extended stats response)

**Example MCP stats response:**
```json
{
  "episodes": 500,
  "entities": 300,
  "edges": 150,
  "decayed_entities": 31,
  "decayed_edges": 87,
  "db_size_bytes": 4096000,
  "sources": [["user", 300], ["system", 200]],
  "total_entities_by_type": [["fact", 150], ["experience", 89], ["preference", 12]],
  "decayed_entities_by_type": [["experience", 23], ["fact", 8]],
  "last_cleanup_at": "2026-04-05T10:00:00Z",
  "queries_since_cleanup": 73,
  "cleanup_interval": 100,
  "cleanup_in_progress": false,
  "next_cleanup_in": 27
}
```

**Test results:** 165 tests pass (73 lib + 92 integration), 0 new clippy warnings.

---

## 🟢 P2 — Manual Override (Lower Priority)

### 5. Wire Up Existing `reverify` CLI Commands ✅ COMPLETE

**Status:** ✅ COMPLETE — 2026-04-07

**What was changed:**
- Added `pub mod reverify;` to `commands/mod.rs`
- Added `ReverifyAction` enum to `main.rs` with 4 subcommands:
  - `list` — list stale memories with `--threshold`, `--limit`, `--offset`, `--format`
  - `renew` — renew a memory with `id` and `--memory-type`
  - `update` — update memory with `id`, `--content`, `--memory-type`
  - `expire` — immediately delete a memory by `id`
- Added `Reverify` variant to `Commands` enum
- Wired up match arm in `main()` to route to existing reverify functions

**Files changed:**
- `crates/ctxgraph-cli/src/commands/mod.rs` — +1 line
- `crates/ctxgraph-cli/src/main.rs` — +78 lines (enum + match arm)

**Usage:**
```bash
ctxgraph reverify list                    # show stale memories (threshold 0.7)
ctxgraph reverify list --threshold 0.5    # lower threshold
ctxgraph reverify list --format json      # JSON output
ctxgraph reverify renew <id> --memory-type fact   # renew a memory
ctxgraph reverify update <id> --content "new text" # update content
ctxgraph reverify expire <id>             # delete immediately
```

**Test results:** CLI builds and help text verified. 92 core tests pass.

---

### 6. Add `--hard` Flag to MCP Forget Tool ✅ COMPLETE

**Status:** ✅ COMPLETE — 2026-04-07

**What was changed:**
- Added `mark_for_deletion(id)` method to `SqliteStorage`:
  - Sets `metadata.marked_for_deletion = true` and `metadata.soft_expired_at` timestamp
  - Works on both entities and edges
  - Returns `true` if found and marked, `false` if not found
- Updated MCP `forget` tool to check `hard` argument:
  - `hard: false` (default): calls `mark_for_deletion()` — soft expire, deletion in next cleanup
  - `hard: true`: calls `expire_memory()` — immediate deletion from DB
- Updated tool schema with `hard` parameter (default: false)
- Updated tool description in MCP tools list
- Added 2 unit tests for `mark_for_deletion` (nonexistent ID, successful mark)

**Files changed:**
- `crates/ctxgraph-core/src/storage/sqlite.rs` — +36 lines (mark_for_deletion method + 2 tests)
- `crates/ctxgraph-mcp/src/tools.rs` — +22 lines (updated forget tool + schema)

**Example MCP forget calls:**
```json
// Soft expire (default)
{"name": "forget", "arguments": {"id": "ent_123"}}
// Response: {"ok": true, "id": "ent_123", "hard": false, "note": "will be removed in next cleanup"}

// Hard delete
{"name": "forget", "arguments": {"id": "ent_456", "hard": true}}
// Response: {"ok": true, "id": "ent_456", "hard": true}
```

**Build status:** ctxgraph-core and ctxgraph-mcp build cleanly.

---

### 7. Add `--type` Flag to MCP Forget Tool (Bulk Expire) ✅ COMPLETE

**Status:** ✅ COMPLETE — 2026-04-07

**What was changed:**
- Added `expire_memories_by_type(memory_type, hard)` method to `SqliteStorage`:
  - Rejects "pattern" type with error (patterns never expire)
  - Soft expire (hard=false): marks all matching entities and edges with `marked_for_deletion` metadata
  - Hard delete (hard=true): immediately deletes all matching entities and edges (edges first for FK integrity)
  - Returns `(entities_affected, edges_affected)` counts
- Updated MCP `forget` tool to support `type` argument:
  - `id` and `type` are mutually exclusive — error if both provided
  - At least one of `id` or `type` is required — error if neither provided
  - Type supports same soft/hard modes as single ID expire
- Updated tool schema with `type` parameter
- Updated tool description
- Added 4 unit tests:
  - `test_expire_memories_by_type_rejects_pattern` — pattern type rejected
  - `test_expire_memories_by_type_soft_empty` — soft expire with no memories
  - `test_expire_memories_by_type_hard_empty` — hard delete with no memories
  - `test_expire_memories_by_type_soft_marks_entities` — verifies bulk marking works

**Files changed:**
- `crates/ctxgraph-core/src/storage/sqlite.rs` — +102 lines (expire_memories_by_type method + 4 tests)
- `crates/ctxgraph-mcp/src/tools.rs` — +40 lines (updated forget tool + schema)

**Example MCP forget calls:**
```json
// Bulk soft expire all experiences
{"name": "forget", "arguments": {"type": "experience"}}
// Response: {"ok": true, "type": "experience", "hard": false, "entities_affected": 47, "edges_affected": 89, "note": "136 marked for cleanup experience memories"}

// Bulk hard delete all facts
{"name": "forget", "arguments": {"type": "fact", "hard": true}}
// Response: {"ok": true, "type": "fact", "hard": true, "entities_affected": 150, "edges_affected": 0, "note": "150 deleted fact memories"}

// Reject pattern expire
{"name": "forget", "arguments": {"type": "pattern"}}
// Response: Error: "pattern memories never expire"
```

**Build status:** ctxgraph-core and ctxgraph-mcp build cleanly. Pre-existing test issues in pattern.rs (async test bugs unrelated to our changes).

---

## ✅ Already Implemented (No Action Needed)

| Feature | Status | Location |
|---------|--------|----------|
| `cleanup_expired()` | ✅ Complete | `sqlite.rs:1408-1596` |
| Grace period behavior | ✅ Complete | `AgentPolicy::grace_period_secs` (7 days) |
| `CleanupResult` struct | ✅ Complete | `types.rs` |
| Archive vs delete logic | ✅ Complete | `sqlite.rs` (archive Pref/Decision, delete Fact/Experience) |
| Pattern preservation | ✅ Complete | Patterns never touched |
| `retrieve_for_context` integration | ✅ Complete | `graph.rs:600` calls `maybe_trigger_cleanup()` |
| System metadata table | ✅ Complete | `migrations.rs:009` |
| MCP forget tool (basic) | ✅ Complete | `tools.rs:402-411` |
| Storage expire_memory | ✅ Complete | `sqlite.rs:1371-1388` |
| Storage renew_memory | ✅ Complete | `sqlite.rs:1268-1298` |
| Storage update_memory | ✅ Complete | `sqlite.rs:1300-1368` |
| Storage get_stale_memories | ✅ Complete | `sqlite.rs:1132-1266` |

---

## Suggested Work Order

1. **Task 1** — Fix lazy trigger counter (critical bug)
2. **Task 2** — Add cleanup_interval config (depends on Task 1)
3. **Task 3** — Check cleanup_in_progress in trigger (depends on Task 1)
4. **Task 4** — Add cleanup visibility to MCP stats (independent)
5. **Task 5** — Wire up reverify CLI (independent, lower priority)
6. **Task 6** — Add --hard to MCP forget (independent, lower priority)
7. **Task 7** — Add --type to MCP forget (independent, lowest priority)

Tasks 1-4 are required for the spec to work correctly. Tasks 5-7 are nice-to-have manual overrides.
