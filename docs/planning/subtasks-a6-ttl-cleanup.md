# A6 TTL Cleanup — Detailed Subtasks

> Each task follows the standard workflow: Analyze → Skills → Research → Design → POC → Test Plan → Implement → Test → Docs

---

## Task 1: Fix Lazy Trigger Counter (P0 — Critical Bug)

**Summary:** Replace `count % 100 == 0` with a resettable `query_count_since_cleanup` counter in system_metadata.

### 1.1 Analyze Current Implementation
- Read `graph.rs::maybe_trigger_cleanup()` (lines 726-745)
- Read `graph.rs::retrieve_for_context()` (lines 593-603)
- Read `sqlite.rs` query counting methods (`query_count()`, atomic counter)
- Read `sqlite.rs` system_metadata methods (`get_system_metadata()`, `set_system_metadata()`)
- Identify all call sites where query_count is incremented
- Document the gap: global counter vs per-cleanup counter

### 1.2 Active Required Skills
- Rust async/sync patterns
- SQLite transaction safety (BEGIN IMMEDIATE, lock handling)
- Atomic counter patterns vs database-backed counters

### 1.3 Research Required Information
- How rusqlite handles concurrent reads/writes to system_metadata
- Whether `query_count_since_cleanup` should be atomic (in-memory) or DB-backed
- Impact of incrementing counter on every retrieval (performance)
- Check if any existing tests depend on the `% 100` behavior

### 1.4 Design the Solution
- Add 3 methods to `SqliteStorage`:
  - `fn increment_query_count_since_cleanup(&self)` — increments DB counter
  - `fn get_query_count_since_cleanup(&self) -> Result<u64>` — reads DB counter
  - `fn reset_query_count_since_cleanup(&self) -> Result<()>` — resets to 0
- Modify `Graph::retrieve_for_context()`:
  - After retrieval succeeds, call `increment_query_count_since_cleanup()`
- Modify `Graph::maybe_trigger_cleanup()`:
  - Read `query_count_since_cleanup` from DB
  - Read `cleanup_interval` from DB (default 100)
  - If `count >= interval`: proceed with cleanup
  - On successful cleanup: call `reset_query_count_since_cleanup()`
- Initialize `query_count_since_cleanup = 0` on first system_metadata access

### 1.5 Create POC
- Write a small test that:
  - Creates a graph
  - Calls `retrieve_for_context()` 105 times
  - Verifies cleanup triggers once (not multiple times)
  - Calls 50 more times — verifies no cleanup
  - Calls 55 more times — verifies cleanup triggers again

### 1.6 Create Test Plan
- Unit tests:
  - `test_increment_query_count` — counter increments correctly
  - `test_reset_query_count` — counter resets to 0
  - `test_trigger_at_interval` — cleanup triggers at exactly `cleanup_interval`
  - `test_no_trigger_before_interval` — no cleanup at count < interval
  - `test_trigger_after_reset` — cleanup triggers again after reset
- Edge cases:
  - Counter wraps around (overflow safety)
  - Multiple concurrent retrievals (race condition check)
  - Cleanup failure does NOT reset counter

### 1.7 Execute the Implementation
- Step 1: Add 3 system_metadata helper methods to `sqlite.rs`
- Step 2: Initialize `query_count_since_cleanup` on first access
- Step 3: Modify `retrieve_for_context()` to increment counter
- Step 4: Rewrite `maybe_trigger_cleanup()` to use DB counter
- Step 5: Reset counter on successful cleanup
- Step 6: Remove or deprecate the old `query_count` atomic if no longer needed

### 1.8 Execute the Testing
- Run unit tests for the 5 new tests
- Run full test suite (`cargo test`)
- Run `cargo clippy` for linting
- Run `cargo build` to ensure no compile errors

### 1.9 Update Documentation
- Update `tasks-a6-ttl-cleanup.md` — mark Task 1 as complete
- Add doc comments to new methods explaining the counter behavior
- If any public API changed, update docstrings

---

## Task 2: Add `cleanup_interval` to system_metadata (P1)

**Summary:** Make cleanup interval configurable via system_metadata instead of hardcoded 100.

### 2.1 Analyze Current Implementation
- Find all hardcoded references to `100` in cleanup-related code
- Check if `AgentPolicy` has any interval-related fields
- Review how other system_metadata fields are initialized and accessed
- Check if there's a config file (`ctxgraph.toml`) that could specify this

### 2.2 Active Required Skills
- Rust configuration patterns
- SQLite JSON storage for system_metadata
- Default value initialization patterns

### 2.3 Research Required Information
- What's the valid range for `cleanup_interval`? (min 1? max 10000?)
- Should interval be changeable at runtime or only at init time?
- Does `ctxgraph.toml` have a section for cleanup config?
- How do other fields in system_metadata handle type conversion (string → int)?

### 2.4 Design the Solution
- Add to system_metadata initialization:
  - Key: `cleanup_interval`, Value: `"100"` (string-encoded integer)
- Add methods to `SqliteStorage`:
  - `fn get_cleanup_interval(&self) -> Result<u64>` — reads from DB, defaults to 100
  - `fn set_cleanup_interval(&self, interval: u64) -> Result<()>` — updates DB
- Modify `maybe_trigger_cleanup()`:
  - Call `get_cleanup_interval()` instead of hardcoded 100
- Validation:
  - If value < 1, use default 100
  - If value > 10000, cap at 10000

### 2.5 Create POC
- Write a test that:
  - Sets `cleanup_interval` to 50
  - Triggers 50 retrievals
  - Verifies cleanup fires at 50 (not 100)

### 2.6 Create Test Plan
- Unit tests:
  - `test_default_cleanup_interval` — returns 100 when not set
  - `test_set_cleanup_interval` — can change the value
  - `test_trigger_respects_custom_interval` — cleanup fires at custom interval
  - `test_invalid_interval_clamped` — values outside range are clamped

### 2.7 Execute the Implementation
- Step 1: Add getter/setter methods to `sqlite.rs`
- Step 2: Add initialization logic for `cleanup_interval` (default 100)
- Step 3: Update `maybe_trigger_cleanup()` to read from DB
- Step 4: Add validation/clamping for interval values

### 2.8 Execute the Testing
- Run 4 new unit tests
- Run full test suite
- Run `cargo clippy`
- Run `cargo build`

### 2.9 Update Documentation
- Update `tasks-a6-ttl-cleanup.md` — mark Task 2 as complete
- Document the configurable interval in method doc comments
- If adding to `ctxgraph.toml`, update example config file

---

## Task 3: Check `cleanup_in_progress` in Lazy Trigger (P1)

**Summary:** The lazy trigger should check `cleanup_in_progress` BEFORE attempting cleanup, not after starting the transaction.

### 3.1 Analyze Current Implementation
- Read `graph.rs::maybe_trigger_cleanup()` — does NOT check `cleanup_in_progress`
- Read `sqlite.rs::cleanup_expired()` — checks `cleanup_in_progress` INSIDE the transaction
- Understand the lock acquisition sequence:
  - Current: retrieve → trigger → BEGIN IMMEDIATE → check lock → ...
  - Desired: retrieve → trigger → check lock → (skip if locked) → BEGIN IMMEDIATE → ...

### 3.2 Active Required Skills
- SQLite locking modes (BEGIN IMMEDIATE, reserved locks)
- Race condition prevention patterns
- Early-exit guard patterns

### 3.3 Research Required Information
- What happens if two retrievals call `maybe_trigger_cleanup()` simultaneously?
- Does the current `cleanup_in_progress` check inside the transaction handle this correctly?
- Is there a performance cost to checking system_metadata on every retrieval?

### 3.4 Design the Solution
- Modify `maybe_trigger_cleanup()`:
  ```rust
  fn maybe_trigger_cleanup(&self) -> Result<()> {
      // Check if cleanup is already in progress
      if let Some(val) = self.storage.get_system_metadata("cleanup_in_progress")? {
          if val == "true" {
              return Ok(()); // skip silently
          }
      }

      let count = self.storage.get_query_count_since_cleanup()?;
      let interval = self.storage.get_cleanup_interval()?;

      if count >= interval {
          let grace = AgentPolicy::default().grace_period_secs;
          let _ = self.storage.cleanup_expired(grace);
      }
      Ok(())
  }
  ```
- The early check prevents unnecessary transaction attempts
- The storage-level check (inside `cleanup_expired`) remains as a safety net

### 3.5 Create POC
- Write a test that simulates concurrent cleanup:
  - Set `cleanup_in_progress = true`
  - Call `maybe_trigger_cleanup()`
  - Verify it returns immediately without attempting cleanup

### 3.6 Create Test Plan
- Unit tests:
  - `test_skip_when_cleanup_in_progress` — trigger returns early
  - `test_proceed_when_not_in_progress` — trigger proceeds normally
  - `test_concurrent_trigger_attempts` — multiple triggers don't duplicate work
- Edge cases:
  - `cleanup_in_progress` key missing (treat as false)
  - `cleanup_in_progress` has invalid value (treat as false)

### 3.7 Execute the Implementation
- Step 1: Add early `cleanup_in_progress` check to `maybe_trigger_cleanup()`
- Step 2: Handle missing/invalid key gracefully (default to false)
- Step 3: Keep the storage-level check as defensive fallback

### 3.8 Execute the Testing
- Run 3 new unit tests
- Run full test suite
- Run `cargo clippy`
- Run `cargo build`

### 3.9 Update Documentation
- Update `tasks-a6-ttl-cleanup.md` — mark Task 3 as complete
- Document the two-level guard (trigger-level + storage-level) in comments

---

## Task 4: Add Cleanup Visibility to MCP Stats Tool (P1)

**Summary:** Extend the MCP `stats` tool to return cleanup state information.

### 4.1 Analyze Current Implementation
- Read `tools.rs::stats()` (lines 356-368) — current response structure
- Read `sqlite.rs::stats()` — what it returns
- Check if `GraphStats` struct has cleanup-related fields
- Check how per-type entity counts are computed (currently only total counts)

### 4.2 Active Required Skills
- JSON-RPC MCP tool patterns
- serde_json serialization
- SQLite aggregation queries (COUNT, GROUP BY)

### 4.3 Research Required Information
- What fields does `GraphStats` currently have?
- Does the storage layer already compute per-type counts?
- How to compute `decayed_entities` by type efficiently?
- What's the MCP tool response schema (any validation)?

### 4.4 Design the Solution
- Extend `GraphStats` in `types.rs` with cleanup fields:
  ```rust
  pub struct GraphStats {
      // existing fields...
      pub last_cleanup_at: Option<String>,
      pub queries_since_cleanup: u64,
      pub cleanup_interval: u64,
      pub cleanup_in_progress: bool,
      pub total_entities_by_type: HashMap<String, u64>,
      pub decayed_entities_by_type: HashMap<String, u64>,
  }
  ```
- Add methods to `SqliteStorage`:
  - `fn get_entity_counts_by_type(&self) -> Result<HashMap<String, u64>>`
  - `fn get_decayed_counts_by_type(&self, grace_period: u64) -> Result<HashMap<String, u64>>`
- Update `sqlite.rs::stats()` to populate new fields
- Update `tools.rs::stats()` to include new fields in JSON response

### 4.5 Create POC
- Write a test that calls `stats()` and verifies:
  - `total_entities_by_type` has correct counts per memory type
  - `last_cleanup_at` is present (or null)
  - `queries_since_cleanup` > 0 after some retrievals

### 4.6 Create Test Plan
- Unit tests:
  - `test_stats_includes_cleanup_fields` — all cleanup fields present
  - `test_entity_counts_by_type` — correct counts per type
  - `test_decayed_counts_by_type` — correct decayed counts
  - `test_next_cleanup_in_calculation` — `next_cleanup_in = interval - queries`
- Integration test:
  - Call MCP `stats` tool and verify JSON structure

### 4.7 Execute the Implementation
- Step 1: Extend `GraphStats` struct with new fields
- Step 2: Add per-type count queries to `sqlite.rs`
- Step 3: Update `stats()` in storage to populate new fields
- Step 4: Update MCP `stats` tool to include new fields in response
- Step 5: Calculate `next_cleanup_in` = max(0, interval - queries)

### 4.8 Execute the Testing
- Run 4 new unit tests
- Run full test suite
- Run `cargo clippy`
- Run `cargo build`

### 4.9 Update Documentation
- Update `tasks-a6-ttl-cleanup.md` — mark Task 4 as complete
- Update MCP tool documentation if any exists

---

## Task 5: Wire Up Existing `reverify` CLI Commands (P2)

**Summary:** The `reverify.rs` file exists with list/renew/update/expire functions but is not registered in mod.rs or main.rs.

### 5.1 Analyze Current Implementation
- Read `commands/reverify.rs` — understand all 4 functions and their option structs
- Read `commands/mod.rs` — see how other commands are exported
- Read `main.rs` — see how other commands are registered in the `Commands` enum
- Check `open_graph()` in mod.rs — reverify commands need graph access

### 5.2 Active Required Skills
- Clap CLI subcommand patterns
- Rust module exports
- Command routing in main()

### 5.3 Research Required Information
- What's the existing CLI command naming convention? (e.g., `ctxgraph entities list`)
- Should reverify be a top-level command or under a subcommand?
- Are there any dependency issues (serde_json, etc.) in reverify.rs?

### 5.4 Design the Solution
- Add to `commands/mod.rs`:
  ```rust
  pub mod reverify;
  ```
- Add to `main.rs` `Commands` enum:
  ```rust
  /// List and manage stale memories for re-verification
  Reverify {
      #[command(subcommand)]
      action: ReverifyAction,
  }
  ```
- Add subcommand enum:
  ```rust
  enum ReverifyAction {
      List { threshold: f64, limit: usize, offset: usize, format: String },
      Renew { id: String, memory_type: String },
      Update { id: String, content: Option<String>, memory_type: Option<String> },
      Expire { id: String },
  }
  ```
- Wire up match arm in `main()`

### 5.5 Create POC
- Build the CLI and run:
  - `ctxgraph reverify list` — shows stale memories table
  - `ctxgraph reverify list --format json` — shows JSON output

### 5.6 Create Test Plan
- Manual tests (CLI doesn't have automated tests typically):
  - `ctxgraph reverify list` — no errors, shows table
  - `ctxgraph reverify renew <id>` — renews memory
  - `ctxgraph reverify update <id> --content "new"` — updates memory
  - `ctxgraph reverify expire <id>` — expires memory
  - Test error cases: invalid ID, invalid memory_type
- `cargo build` — no compile errors

### 5.7 Execute the Implementation
- Step 1: Add `pub mod reverify;` to `commands/mod.rs`
- Step 2: Add `Reverify` variant with subcommand enum to `main.rs`
- Step 3: Add match arm in `main()` to route to reverify functions
- Step 4: Test compilation and basic execution

### 5.8 Execute the Testing
- `cargo build` — must compile cleanly
- Manual test runs with a real graph.db
- `cargo clippy` — no warnings

### 5.9 Update Documentation
- Update `tasks-a6-ttl-cleanup.md` — mark Task 5 as complete
- Add CLI help text is handled by clap doc comments

---

## Task 6: Add `--hard` Flag to MCP Forget Tool (P2)

**Summary:** MCP forget tool currently marks for cleanup. Add `hard=true` for immediate deletion.

### 6.1 Analyze Current Implementation
- Read `tools.rs::forget()` (lines 402-411) — current implementation
- Read `graph.rs::expire_memory()` — immediate delete
- Check if there's a "mark for cleanup" vs "immediate delete" distinction already

### 6.2 Active Required Skills
- MCP tool argument parsing
- Error handling for "not found" cases

### 6.3 Research Required Information
- Current `forget()` calls `graph.expire_memory(id)` — which does immediate delete, NOT mark
- Breakdown spec says forget should "mark for deletion" by default, delete immediately with `--hard`
- Need to clarify: what does "mark for deletion" mean? (Set decay_score to 0? Set a flag?)

### 6.4 Design the Solution
- Current `forget()` already does immediate delete (calls `expire_memory`)
- Rename current behavior to `hard` mode
- Add soft mode: set `decay_score = 0` in metadata (or set `marked_for_deletion = true`)
- MCP tool args:
  ```json
  { "id": "ent_123", "hard": false }  // default: mark for cleanup
  { "id": "ent_123", "hard": true }   // immediate delete
  ```
- Add method to `SqliteStorage`:
  - `fn mark_for_deletion(&self, id: &str) -> Result<()>` — sets metadata flag

### 6.5 Create POC
- Write a test that:
  - Calls `forget(id, hard=false)` — verifies memory is marked but still queryable
  - Calls `forget(id, hard=true)` — verifies memory is gone

### 6.6 Create Test Plan
- Unit tests:
  - `test_forget_soft_marks_memory` — metadata flag set
  - `test_forget_hard_deletes_memory` — memory removed from DB
  - `test_forget_not_found` — graceful error handling

### 6.7 Execute the Implementation
- Step 1: Add `mark_for_deletion()` method to `sqlite.rs`
- Step 2: Modify `tools.rs::forget()` to check `hard` argument
- Step 3: Call appropriate method based on flag

### 6.8 Execute the Testing
- Run 3 new unit tests
- Run full test suite
- Run `cargo clippy`
- Run `cargo build`

### 6.9 Update Documentation
- Update `tasks-a6-ttl-cleanup.md` — mark Task 6 as complete
- Update MCP tool descriptions in any docs

---

## Task 7: Add `--type` Flag to MCP Forget Tool (P2 — Bulk Expire)

**Summary:** Support bulk expiring all memories of a specific type.

### 7.1 Analyze Current Implementation
- Check if there's any bulk operation pattern in sqlite.rs
- Review `expire_memories_by_type` requirements — which types are allowed?
- Patterns should NEVER be expired — must be excluded

### 7.2 Active Required Skills
- SQLite batch update/delete operations
- Transaction safety for bulk operations

### 7.3 Research Required Information
- How many entities could this affect? (performance concern for large graphs)
- Should bulk expire be soft (mark) or hard (delete)?
- Should there be a limit/max count parameter?

### 7.4 Design the Solution
- Add to `SqliteStorage`:
  ```rust
  fn mark_memories_by_type_for_deletion(&self, memory_type: &str) -> Result<u64>;
  fn expire_memories_by_type(&self, memory_type: &str, hard: bool) -> Result<u64>;
  ```
- MCP tool args:
  ```json
  { "type": "experience" }  // mark all experiences for cleanup
  { "type": "experience", "hard": true }  // delete all experiences immediately
  ```
- Pattern type is rejected with an error
- Returns count of affected memories

### 7.5 Create POC
- Write a test that:
  - Creates 5 experience entities
  - Calls `forget(type="experience")`
  - Verifies all 5 are marked/deleted
  - Verifies patterns are unaffected

### 7.6 Create Test Plan
- Unit tests:
  - `test_bulk_expire_marks_memories` — correct count marked
  - `test_bulk_hard_expire_deletes_memories` — correct count deleted
  - `test_bulk_expire_patterns_rejected` — error when targeting patterns
  - `test_bulk_expire_mixed_types` — only target type affected
- Edge cases:
  - No memories of type → returns 0
  - Very large graph → transaction doesn't timeout

### 7.7 Execute the Implementation
- Step 1: Add bulk expire methods to `sqlite.rs`
- Step 2: Modify `tools.rs::forget()` to handle `type` argument
- Step 3: Add validation (reject pattern type)
- Step 4: Return count in response

### 7.8 Execute the Testing
- Run 4 new unit tests
- Run full test suite
- Run `cargo clippy`
- Run `cargo build`

### 7.9 Update Documentation
- Update `tasks-a6-ttl-cleanup.md` — mark Task 7 as complete
- Document bulk expire behavior

---

## Dependency Graph

```
Task 1 (Fix lazy counter)
  ├── Task 2 (cleanup_interval)  — depends on Task 1 infrastructure
  ├── Task 3 (cleanup_in_progress check) — depends on Task 1 infrastructure
  └── Task 4 (MCP stats) — can read cleanup fields added by Tasks 1-3

Task 5 (Wire reverify CLI) — independent
Task 6 (MCP forget --hard) — independent
Task 7 (MCP forget --type) — depends on Task 6 (shares forget tool changes)

Recommended order: 1 → 2 → 3 → 4 → 5 → 6 → 7
```

## Summary Table

| Task | Subtasks | Est. Files Changed | Priority |
|------|----------|-------------------|----------|
| 1. Fix lazy counter | 9 subtasks | 2-3 files | 🔴 P0 |
| 2. cleanup_interval | 9 subtasks | 2 files | 🟡 P1 |
| 3. cleanup_in_progress check | 9 subtasks | 1 file | 🟡 P1 |
| 4. MCP stats visibility | 9 subtasks | 3-4 files | 🟡 P1 |
| 5. Wire reverify CLI | 9 subtasks | 2 files | 🟢 P2 |
| 6. MCP forget --hard | 9 subtasks | 2 files | 🟢 P2 |
| 7. MCP forget --type | 9 subtasks | 2 files | 🟢 P2 |

**Total: 63 subtasks across 7 tasks**
