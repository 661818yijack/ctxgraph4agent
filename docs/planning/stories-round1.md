# ctxgraph4agent — Round 1 User Stories

> Memory lifecycle: Store -> TTL -> Forget -> Decay -> Re-verify -> Compress -> Budget -> Learn

## Phase A: TTL + Decay (Foundation)

Stories A1-A5 establish the core memory lifecycle. Every subsequent phase depends on these fields and computations existing first.

---

## A1: Add ttl and memory_type fields to Entity and Edge

**Phase:** A
**Priority:** P0
**Effort:** M
**Depends on:** none

### Description
Add `memory_type` (Fact, Pattern, Experience, Preference, Decision) and `ttl` (Option<Duration>) fields to both Entity and Edge structs. These fields are persisted in SQLite via a new migration (003). Episodes also get `memory_type` to classify the source. The `memory_type` defaults based on `entity_type` mapping (e.g. Decision entity -> Decision memory_type, Component -> Fact) but can be overridden explicitly.

### Acceptance Criteria
1. `Entity` struct has `memory_type: MemoryType` and `ttl: Option<Duration>` fields with sensible defaults (Fact -> 90d, Pattern -> None/never, Experience -> 14d, Preference -> 30d, Decision -> 90d)
2. `Edge` struct has `memory_type: MemoryType` and `ttl: Option<Duration>` fields defaulting to match its source entity's type
3. New `MemoryType` enum with variants `Fact`, `Pattern`, `Experience`, `Preference`, `Decision` implements Serialize/Deserialize and Display
4. Migration 003 adds `memory_type TEXT NOT NULL DEFAULT 'Fact'` and `ttl_seconds INTEGER` columns to `entities` and `edges` tables, plus `memory_type` to `episodes`
5. All existing `insert_entity` and `insert_edge` paths write the new fields; all read paths (`get_entity`, `list_entities`, `get_edge`, etc.) populate them
6. `map_entity_row` and `map_edge_row` helper functions updated to read new columns

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `MemoryType` enum, add fields to `Entity` and `Edge`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 003
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — update `insert_entity`, `insert_edge`, `map_entity_row`, `map_edge_row`, `get_entity`, `get_episode`, `list_episodes`
- **New types/functions:** `MemoryType` enum, `MemoryType::default_for_entity_type(&str) -> MemoryType`, `MemoryType::default_ttl(&self) -> Option<Duration>`
- **Config changes:** none

### Test Plan
- Unit: `MemoryType::default_for_entity_type("Decision")` returns `Decision`
- Unit: `MemoryType::default_ttl(&Fact)` returns `Some(Duration::from_secs(90 * 86400))`
- Unit: `MemoryType::default_ttl(&Pattern)` returns `None`
- Integration: insert entity with no explicit ttl, read it back, verify ttl is 90 days for Fact
- Integration: migration 003 applied to existing DB, existing rows get `memory_type='Fact'` and `ttl_seconds` populated via UPDATE in migration SQL
- Integration: entity with `ttl=None` (Pattern) persists and reads back as `None`

---

## A2: Implement decay_score computation

**Phase:** A
**Priority:** P0
**Effort:** M
**Depends on:** A1

### Description
Implement the `decay_score` function that computes freshness at query time (not stored). The function takes a node's memory_type, ttl, base confidence, age (now - created_at), and returns a score in [0.0, 1.0]. Three decay functions: exponential for facts (half-life = TTL/2), constant 1.0 for patterns, and linear drop to 0.0 at TTL for experiences. Preferences and decisions use exponential like facts. If ttl is None (patterns), decay_score always returns the base confidence.

### Acceptance Criteria
1. `decay_score` function computes `base_confidence * decay_function(age, memory_type, ttl)` returning f64 in [0.0, 1.0]
2. For Fact type: exponential decay with half-life = ttl/2 — a fact at age=0 scores 1.0, at age=ttl/2 scores 0.5, at age=ttl scores 0.25
3. For Pattern type: constant decay — always returns `base_confidence` regardless of age
4. For Experience type: linear decay — at age=0 scores 1.0, at age=ttl scores 0.0, linearly interpolated
5. For Preference and Decision: exponential decay same as Fact
6. If age > ttl (expired), the score is 0.0 for all types except Pattern (which never expires)

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `pub fn decay_score(&self, base_confidence: f64, created_at: DateTime<Utc>) -> f64` on `MemoryType`
  - `crates/ctxgraph-core/src/lib.rs` — re-export if needed
- **New types/functions:** `MemoryType::decay_score`, helper `decay_exponential(age, half_life)`, `decay_linear(age, ttl)`, `decay_constant(confidence)`
- **Config changes:** none

### Test Plan
- Unit: `Fact.decay_score(1.0, now)` with age=0 returns 1.0
- Unit: `Fact.decay_score(1.0, 45 days ago)` with ttl=90d returns ~0.5
- Unit: `Pattern.decay_score(0.8, 365 days ago)` returns 0.8 exactly
- Unit: `Experience.decay_score(1.0, 7 days ago)` with ttl=14d returns 0.5
- Unit: `Fact.decay_score(1.0, 100 days ago)` with ttl=90d returns 0.0 (expired)
- Unit: `Experience.decay_score(1.0, 15 days ago)` with ttl=14d returns 0.0 (expired)
- Property: all decay functions return values in [0.0, 1.0]

---

## A3: Add usage_count and last_recalled_at tracking

**Phase:** A
**Priority:** P1
**Effort:** M
**Depends on:** A1

### Description
Add `usage_count: u32` and `last_recalled_at: Option<DateTime<Utc>>` to Entity and Edge. These fields track how often a memory is recalled and when it was last used. They are updated when memories appear in search/retrieval results that are consumed by the agent. A new `touch_memory` method increments usage_count and sets last_recalled_at to now. These fields feed into re-verification (Phase C) and budget ranking (A4).

### Acceptance Criteria
1. `Entity` and `Edge` structs have `usage_count: u32` (default 0) and `last_recalled_at: Option<DateTime<Utc>>` (default None)
2. Migration 004 adds `usage_count INTEGER NOT NULL DEFAULT 0` and `last_recalled_at TEXT` columns to `entities` and `edges` tables
3. `Storage::touch_entity(id: &str)` increments usage_count and sets last_recalled_at to Utc::now() in a single UPDATE
4. `Storage::touch_edge(id: &str)` same behavior for edges
5. `Storage::get_usage_stats()` returns total counts grouped by memory_type for dashboard/monitoring
6. All read paths (get_entity, list_entities, get_edge, etc.) populate the new fields

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add fields to `Entity` and `Edge`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 004
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `touch_entity`, `touch_edge`, `get_usage_stats`; update all read paths
- **New types/functions:** `Storage::touch_entity(&self, id: &str) -> Result<()>`, `Storage::touch_edge(&self, id: &str) -> Result<()>`, `Storage::get_usage_stats(&self) -> Result<Vec<(MemoryType, u64, u64)>>`
- **Config changes:** none

### Test Plan
- Integration: insert entity, call `touch_entity` 3 times, read entity back, verify `usage_count == 3`
- Integration: call `touch_entity`, verify `last_recalled_at` is set to recent timestamp
- Integration: migration 004 applied to existing DB, existing rows get `usage_count=0` and `last_recalled_at=NULL`
- Unit: `touch_entity` on nonexistent id returns error

---

## A4: Budget-aware retrieval ranking

**Phase:** A
**Priority:** P0
**Effort:** L
**Depends on:** A1, A2, A3

### Description
Implement a new `retrieve_for_context` method on Storage that ranks memories by `freshness * relevance * recency_bonus` and enforces a hard 20k token budget. The method takes a query string, agent policy (from A5), and budget limit. It retrieves candidates via FTS5 + graph traversal, computes decay_score for each, ranks them, then greedily fills the budget. Patterns are always included (they're small and permanent). Token counting uses a simple estimator (~4 chars per token). Results are capped at budget and returned as a `Vec<RankedMemory>`.

### Acceptance Criteria
1. `Storage::retrieve_for_context(query, policy, budget_tokens)` returns `Vec<RankedMemory>` sorted by composite score
2. Composite score = `decay_score * fts_relevance * (1.0 + 0.1 * ln(1 + usage_count))` — frequent usage gets a small bonus
3. Patterns (memory_type=Pattern) are always included regardless of score, unless budget is exceeded
4. Total token count of all returned memories does not exceed the budget (default 20,000 tokens)
5. Token estimation uses `text.len() / 4` as a simple character-to-token ratio
6. If no memories match the query, returns empty vec (not an error)

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `RankedMemory` struct, `MemoryPolicy` struct
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `retrieve_for_context` method
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::retrieve_for_context` passthrough
- **New types/functions:**
  - `RankedMemory { memory_type: MemoryType, content: String, score: f64, entity_id: Option<String>, edge_id: Option<String> }`
  - `MemoryPolicy { memory_budget_tokens: usize, agent_name: String }`
  - `MemoryPolicy::default() -> MemoryPolicy` (budget = 20_000)
  - `estimate_tokens(text: &str) -> usize` (text.len() / 4)
- **Config changes:** none (policy loaded from config in A5)

### Test Plan
- Unit: `estimate_tokens("hello world")` returns ~3
- Integration: insert 50 entities, retrieve_for_context returns <= 20k tokens worth
- Integration: patterns are included even if they have low relevance score
- Integration: if budget is 100 tokens, only 1-2 small memories returned
- Integration: expired memories (decay_score=0.0) are not returned unless explicitly queried
- Property: sum of estimate_tokens for all results <= budget_tokens

---

## A5: Per-agent memory policies via ctxgraph.toml [policies] section

**Phase:** A
**Priority:** P1
**Effort:** M
**Depends on:** A1, A4

### Description
Extend `ctxgraph.toml` with a `[policies.<agent_name>]` section that configures TTL, budget, and compression settings per agent. Each policy specifies TTLs per memory_type, memory_budget_tokens, compress_after duration, and max_episodes. The `MemoryPolicyConfig` struct is loaded alongside the existing `ExtractionSchema`. A `set_policy` MCP tool allows runtime policy changes. The `retrieve_for_context` method (A4) uses the active agent's policy.

### Acceptance Criteria
1. `ctxgraph.toml` supports `[policies.<name>]` with keys: `facts_ttl`, `experiences_ttl`, `patterns_ttl` (always "never"), `preferences_ttl`, `decisions_ttl`, `memory_budget_tokens`, `compress_after`, `max_episodes`
2. `MemoryPolicyConfig` struct deserializes from TOML with defaults matching the assistant policy from CLAUDE.md (facts=90d, experiences=14d, patterns=never, preferences=30d, decisions=90d, budget=20000)
3. `ctxgraph.toml.example` updated with the policies section
4. New MCP tool `set_policy` allows changing TTL/budget at runtime for the active agent
5. `Graph::init` loads policies from `.ctxgraph/ctxgraph.toml` if present, falls back to defaults
6. Invalid policy values (e.g. negative TTL, budget < 1000) produce a clear error

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-extract/src/schema.rs` — add `MemoryPolicyConfig` struct, extend TOML parsing
  - `crates/ctxgraph-core/src/types.rs` — add `MemoryPolicyConfig` (or move to core since it's used at retrieval time)
  - `crates/ctxgraph-core/src/graph.rs` — load policy config in `init` and `open`, store as field on `Graph`
  - `crates/ctxgraph-mcp/src/tools.rs` — add `set_policy` tool handler and tool definition
  - `ctxgraph.toml.example` — add policies section
- **New types/functions:** `MemoryPolicyConfig`, `MemoryPolicyConfig::load(path: &Path)`, `MemoryPolicyConfig::default()`, `MemoryPolicyConfig::for_agent(&self, agent_name: &str) -> Option<&AgentPolicy>`
- **Config changes:** new `[policies.<agent>]` section in ctxgraph.toml

### Test Plan
- Unit: parse a TOML string with `[policies.programming]` section, verify TTL values
- Unit: `MemoryPolicyConfig::default()` returns assistant policy defaults
- Unit: invalid TTL string like "banana" returns `SchemaError::Parse`
- Integration: `Graph::init` creates DB and loads default policy
- Integration: MCP `set_policy` tool changes budget, subsequent retrievals use new budget
- Unit: policy with budget < 1000 returns validation error

---

## Phase B: Compress (Size Control)

Stories B1-B4 implement the compression pipeline that keeps the graph small while preserving patterns.

---

## B1: Episode compression pipeline

**Phase:** B
**Priority:** P0
**Effort:** L
**Depends on:** A1, A3

### Description
Implement a compression pipeline that batches old episodes into a single summary node. Given a set of episode IDs and an optional time range, the pipeline creates a new "compressed episode" entity with a generated summary, marks the source episodes with `compression_id` linking them to the summary, and sets their decay to accelerated. The summary is generated via extraction of shared entities/relations rather than LLM call (keeping it fast). The compressed episode has `memory_type: Pattern` (since it's a learned summary, not a raw fact).

### Acceptance Criteria
1. `Storage::compress_episodes(episode_ids: &[String], summary: String) -> Result<String>` creates a new episode with the summary content, `memory_type: Pattern`, and returns its ID
2. Each source episode gets `compression_id` set to the new summary episode's ID via an UPDATE
3. A new `compression_groups` table tracks `(compression_id, source_episode_id)` for audit
4. The compressed summary episode has all entities from source episodes merged as `episode_entities` links
5. `compression_id: Option<String>` field added to `Episode` struct
6. Compressed episodes are queryable via normal search; source episodes remain until decayed

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `compression_id: Option<String>` to `Episode`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 005 (compression_groups table, compression_id on episodes)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `compress_episodes`, update episode read/write paths
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::compress_episodes` passthrough
- **New types/functions:**
  - `Storage::compress_episodes(&self, episode_ids: &[String], summary: &str) -> Result<String>`
  - `Storage::get_compression_group(&self, compression_id: &str) -> Result<Vec<String>>`
  - `Storage::list_uncompressed_episodes(&self, before: DateTime<Utc>, memory_type: MemoryType) -> Result<Vec<Episode>>`
- **Config changes:** none

### Test Plan
- Integration: insert 5 episodes, compress them, verify summary episode created with 5 source links
- Integration: source episodes have `compression_id` set to new summary ID
- Integration: `list_uncompressed_episodes` returns only ungrouped episodes
- Integration: compressed episode has merged entity links from all source episodes
- Unit: compressing empty episode_ids returns error

---

## B2: Relationship inheritance from compressed nodes

**Phase:** B
**Priority:** P1
**Effort:** M
**Depends on:** B1

### Description
When episodes are compressed into a summary, the edges (relationships) from the source episodes should be inherited by the summary node. Duplicate edges (same source_id + target_id + relation) are merged into a single edge with accumulated confidence. The inherited edges retain their original memory_type but get a metadata flag `{"inherited_from": "compression_id"}`. Edges that become redundant (both endpoints now linked to the compressed summary) are invalidated.

### Acceptance Criteria
1. `compress_episodes` also copies/merges edges from source episodes to the summary episode
2. Duplicate edges (same source + target + relation) are merged: confidence = max of source confidences, metadata lists source edge IDs
3. Inherited edges get metadata `{"inherited_from": "<compression_id>", "source_edges": ["id1", "id2"]}`
4. `Storage::get_edges_for_entity` on the compressed summary returns all inherited edges
5. Original edges from source episodes are NOT deleted (they decay naturally per their TTL)
6. Edge merging is idempotent — running compression on the same group twice is safe

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — extend `compress_episodes` to handle edge inheritance, add `merge_edges` helper
- **New types/functions:** `Storage::merge_edges_for_compression(&self, compression_id: &str, episode_ids: &[String]) -> Result<usize>`
- **Config changes:** none

### Test Plan
- Integration: compress 3 episodes with overlapping entities, verify summary has merged edges (deduplicated)
- Integration: merged edge has max confidence of source edges
- Integration: merged edge metadata contains `inherited_from` field
- Integration: original edges still exist and are queryable
- Unit: compressing episodes with no edges produces summary with no inherited edges

---

## B3: Compression triggers (time-based and size-based)

**Phase:** B
**Priority:** P1
**Effort:** M
**Depends on:** B1, A5

### Description
Add automatic compression triggers that run at query time (lazy, not daemon). Two trigger types: time-based (compress episodes older than `compress_after` days) and size-based (compress when episode count exceeds `max_episodes`). The trigger checks are called at the start of `retrieve_for_context` — if conditions are met, it identifies eligible episode groups (same source or same entity cluster) and compresses them. Triggers respect per-agent policy settings from A5.

### Acceptance Criteria
1. `CompressionTrigger` struct evaluates whether compression should run given current state and policy
2. Time-based trigger: if any ungrouped episodes are older than `compress_after` days, group them by source and compress
3. Size-based trigger: if total ungrouped episode count exceeds `max_episodes`, compress oldest batch until under limit
4. `Storage::get_compressible_episodes(before: DateTime<Utc>) -> Result<Vec<Episode>>` finds candidates
5. Triggers are lazy — evaluated at query time, not as background daemon
6. Compression respects memory_type: only experiences and decisions are compressible; facts and patterns are not

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `CompressionTrigger` struct
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `get_compressible_episodes`, update `retrieve_for_context` to call triggers
  - `crates/ctxgraph-core/src/graph.rs` — integrate trigger checks into retrieval path
- **New types/functions:**
  - `CompressionTrigger { compress_after: Duration, max_episodes: usize }`
  - `CompressionTrigger::should_compress(&self, episode_count: usize, oldest_episode_age: Duration) -> bool`
  - `Graph::auto_compress(&self, policy: &MemoryPolicyConfig) -> Result<Option<CompressionResult>>`
- **Config changes:** `compress_after` and `max_episodes` in `[policies.<agent>]`

### Test Plan
- Unit: `CompressionTrigger::should_compress` with 100 episodes and max=50 returns true
- Unit: `CompressionTrigger::should_compress` with 10 episodes and max=50 returns false
- Unit: time-based trigger fires for episodes older than compress_after
- Integration: insert 60 episodes, trigger compresses oldest 10 to get under max_episodes=50
- Integration: experiences are compressed but facts are not
- Integration: trigger is a no-op when no episodes meet criteria

---

## B4: Compression CLI command and MCP tool

**Phase:** B
**Priority:** P1
**Effort:** S
**Depends on:** B1, B3

### Description
Expose the compression pipeline via a CLI subcommand (`ctxgraph compress`) and an MCP tool (`compress`). Both allow manual triggering of compression with options for dry-run, source filter, and force mode. The CLI shows a human-readable summary of what would be/was compressed. The MCP tool returns structured JSON with compression_id and affected episode count.

### Acceptance Criteria
1. CLI: `ctxgraph compress` runs auto-compress with default policy and shows results
2. CLI: `ctxgraph compress --dry-run` shows what would be compressed without doing it
3. CLI: `ctxgraph compress --source meeting` compresses only episodes from "meeting" source
4. MCP: `compress` tool accepts `{source?: string, dry_run?: boolean}` and returns `{compression_id, episodes_compressed, tokens_saved}`
5. CLI: `ctxgraph compress --force` ignores policy thresholds and compresses all eligible episodes
6. Both CLI and MCP output include before/after episode count

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-cli/src/commands/compress.rs` — new command module
  - `crates/ctxgraph-cli/src/commands/mod.rs` — register compress module
  - `crates/ctxgraph-cli/src/main.rs` — add `Compress` subcommand variant
  - `crates/ctxgraph-mcp/src/tools.rs` — add `compress` tool handler and definition
- **New types/functions:** `commands::compress::run(dry_run, source, force)`, MCP tool handler `ToolContext::compress`
- **Config changes:** none

### Test Plan
- Integration: `ctxgraph compress --dry-run` on empty DB shows "nothing to compress"
- Integration: `ctxgraph compress` on DB with 60 episodes compresses and reports count
- Integration: MCP `compress` tool returns valid JSON with compression_id
- Integration: `ctxgraph compress --source nonexistent` returns "no episodes found for source"

---

## Phase C: Re-verify (Quality Maintenance)

Stories C1-C4 ensure memories stay accurate through contradiction detection and TTL renewal.

---

## C1: Passive re-verification (detect contradictions at write time)

**Phase:** C
**Priority:** P0
**Effort:** L
**Depends on:** A1, A2, A3

### Description
When a new episode is ingested, the system checks existing facts for contradictions. If a new fact conflicts with a stored one (same entity pair + same relation type but different fact value), the old edge is invalidated and the new one takes precedence. Contradiction detection uses entity name + relation as the key — if an existing edge says "Alice chose PostgreSQL" and a new episode says "Alice chose MySQL", the old edge is invalidated with `valid_until = now`. Contradicted edges get metadata `{"contradicted_by": "<new_episode_id>"}`.

### Acceptance Criteria
1. `Storage::check_contradictions(&self, new_edges: &[Edge]) -> Result<Vec<Contradiction>>` scans for conflicts
2. A contradiction is detected when: same source entity + same relation type, but different target entity or fact value
3. When contradiction found, the old edge is invalidated (`valid_until = now`) and metadata updated
4. `Contradiction` struct records `{old_edge_id, new_edge_id, entity_name, relation, old_value, new_value}`
5. Contradiction invalidation is called automatically during `Graph::add_episode` after extraction
6. Invalidated edges are no longer returned by `get_current_edges_for_entity` but remain in history

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `Contradiction` struct
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `check_contradictions`, `invalidate_contradicted`
  - `crates/ctxgraph-core/src/graph.rs` — call contradiction check in `add_episode`
- **New types/functions:**
  - `Contradiction { old_edge_id: String, new_edge_id: String, entity_name: String, relation: String, old_value: String, new_value: String }`
  - `Storage::check_contradictions(&self, edges: &[Edge]) -> Result<Vec<Contradiction>>`
  - `Graph::add_episode_with_contradiction_check(&self, episode: Episode) -> Result<(EpisodeResult, Vec<Contradiction>)>`
- **Config changes:** none

### Test Plan
- Integration: insert "Alice chose PostgreSQL", then insert "Alice chose MySQL", verify first edge invalidated
- Integration: invalidated edge has `valid_until` set and `contradicted_by` in metadata
- Integration: insert "Alice chose PostgreSQL" twice — no contradiction (same fact)
- Integration: `get_current_edges_for_entity` returns only the newer edge
- Unit: contradiction check on empty graph returns empty vec
- Integration: EpisodeResult includes contradiction count

---

## C2: Implicit TTL renewal (recalled and used -> auto-renew)

**Phase:** C
**Priority:** P1
**Effort:** M
**Depends on:** A1, A3

### Description
When a memory is recalled via `retrieve_for_context` and its content is actually used (appears in the context sent to the agent), its TTL is implicitly renewed. Renewal resets the effective age to 0 for decay calculation purposes by updating `created_at` to `Utc::now()`. This is gated by a `max_renewals` policy setting (default 5) to prevent memories from living forever through constant renewal. Only Facts and Preferences are eligible for renewal; Experiences are not (they decay linearly and are meant to be forgotten).

### Acceptance Criteria
1. `Storage::renew_memory(id: &str, memory_type: MemoryType) -> Result<bool>` updates `created_at` to now if renewal is allowed
2. Renewal only applies to `Fact` and `Preference` memory types (not Experience, Pattern, or Decision)
3. Renewal count tracked via `usage_count` — if `usage_count > max_renewals`, renewal is denied
4. `MemoryPolicyConfig` has `max_renewals: usize` field (default 5)
5. Renewal returns false (no-op) if memory is already expired (decay_score = 0.0)
6. `retrieve_for_context` automatically calls `renew_memory` for each returned memory

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `max_renewals` to `MemoryPolicyConfig` / `AgentPolicy`
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `renew_memory`, integrate into `retrieve_for_context`
- **New types/functions:** `Storage::renew_memory(&self, id: &str, table: MemoryTable) -> Result<bool>`
- **Config changes:** `max_renewals = 5` in `[policies.<agent>]`

### Test Plan
- Integration: insert fact, recall it, verify `created_at` updated to recent time
- Integration: recall same fact 6 times (max_renewals=5), 6th recall does not renew
- Integration: recall experience — no renewal happens
- Integration: recall pattern — no renewal happens (patterns never expire anyway)
- Unit: renew expired memory (decay_score=0.0) returns false
- Integration: after renewal, decay_score recalculates as if the memory is fresh

---

## C3: Active re-verification (surface stale memories for confirmation)

**Phase:** C
**Priority:** P2
**Effort:** M
**Depends on:** A1, A2, A3

### Description
Implement a `get_stale_memories` method that surfaces memories approaching TTL expiration (decay_score < 0.3) for active re-verification. The agent (or user via CLI/MCP) can review these and choose to renew, update, or let them expire. Stale memories are returned with their current content and a suggested action based on memory_type. This is opt-in — only called explicitly, not automatically.

### Acceptance Criteria
1. `Storage::get_stale_memories(threshold: f64, limit: usize) -> Result<Vec<StaleMemory>>` returns memories with decay_score < threshold
2. `StaleMemory` struct includes the memory content, entity/edge info, decay_score, age, and suggested action (renew/update/expire)
3. Suggested action: Facts -> "verify or update", Preferences -> "confirm with user", Experiences -> "let expire", Patterns -> never stale
4. MCP tool `reverify` returns stale memories with a prompt for the agent to act on
5. CLI: `ctxgraph reverify list` shows stale memories in a human-readable table
6. CLI: `ctxgraph reverify renew <id>` explicitly renews a memory, bypassing max_renewals

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `StaleMemory`, `StaleAction` enum
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `get_stale_memories` (queries entities + edges with decay check)
  - `crates/ctxgraph-mcp/src/tools.rs` — add `reverify` tool handler and definition
  - `crates/ctxgraph-cli/src/commands/reverify.rs` — new command module
  - `crates/ctxgraph-cli/src/main.rs` — add `Reverify` subcommand
- **New types/functions:**
  - `StaleMemory { id: String, memory_type: MemoryType, content: String, decay_score: f64, age_days: u64, suggested_action: StaleAction }`
  - `StaleAction` enum: `Renew`, `Update`, `Expire`
  - `Storage::get_stale_memories(&self, threshold: f64, limit: usize) -> Result<Vec<StaleMemory>>`
- **Config changes:** none

### Test Plan
- Integration: insert fact 80 days ago (ttl=90d), verify it appears in stale list with decay < 0.3
- Integration: insert pattern 365 days ago, verify it does NOT appear in stale list
- Integration: `ctxgraph reverify list` shows at least one stale memory
- Integration: `ctxgraph reverify renew <id>` updates created_at
- Unit: `get_stale_memories` with threshold=0.0 returns no results (nothing below 0)
- Integration: MCP `reverify` tool returns JSON array of stale memories

---

## C4: Re-verify CLI command and MCP tool

**Phase:** C
**Priority:** P2
**Effort:** S
**Depends on:** C1, C2, C3

### Description
Finalize the re-verification CLI and MCP interface. This story wires up all the C1-C3 functionality into a cohesive command structure. CLI gets `ctxgraph reverify` with subcommands `list`, `renew <id>`, `update <id>`, `expire <id>`. MCP gets a unified `reverify` tool that can list stale or take action on a specific memory. Also adds a `forget` MCP tool to manually expire a memory immediately.

### Acceptance Criteria
1. CLI: `ctxgraph reverify list --threshold 0.3 --limit 20` lists stale memories with decay_score
2. CLI: `ctxgraph reverify renew <id>` renews a specific memory (resets created_at)
3. CLI: `ctxgraph reverify expire <id>` immediately invalidates a memory (sets decay_score to 0)
4. MCP: `reverify` tool with `action: "list" | "renew" | "expire"` and `id` for targeted actions
5. MCP: `forget` tool expires a memory by ID with `{"id": "..."}` input
6. `ctxgraph stats` output includes re-verification stats: total stale, total renewed, total expired

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-cli/src/commands/reverify.rs` — add renew, update, expire subcommands
  - `crates/ctxgraph-cli/src/commands/mod.rs` — register reverify module
  - `crates/ctxgraph-cli/src/main.rs` — add `Reverify` enum with action subcommands
  - `crates/ctxgraph-mcp/src/tools.rs` — finalize `reverify` and add `forget` tool definitions
- **New types/functions:** `commands::reverify::run(action)`, MCP `ToolContext::forget`
- **Config changes:** none

### Test Plan
- Integration: `ctxgraph reverify list` on DB with stale memories returns non-empty
- Integration: `ctxgraph reverify renew <id>` then `reverify list` — renewed memory no longer stale
- Integration: `ctxgraph reverify expire <id>` — memory no longer returned by any search
- Integration: MCP `forget` tool with valid ID returns success
- Integration: MCP `forget` with invalid ID returns error
- Integration: `ctxgraph stats` shows re-verification metrics

---

## Phase D: Learn (The Differentiator)

Stories D1-D4 implement the skills layer that makes agents genuinely better over time.

---

## D1: Pattern extraction from compressed experiences

**Phase:** D
**Priority:** P1
**Effort:** L
**Depends on:** B1, B2, A1

### Description
Implement a pattern extraction engine that analyzes compressed episode groups to find recurring behaviors, successful approaches, and anti-patterns. The engine operates on the entity/edge graph structure: it looks for frequently co-occurring entity types, common relation patterns, and repeated sequences across compression groups. Extracted patterns are stored as `memory_type: Pattern` entities with edges linking to the source compression groups. No LLM call required — purely structural analysis.

### Acceptance Criteria
1. `PatternExtractor` struct analyzes a set of compression groups and extracts recurring subgraphs
2. Extraction finds patterns: same entity type appearing in >3 compression groups, same relation triplet (entity_a, relation, entity_b) repeated, same entity pair with different relations (rich context)
3. Extracted patterns stored as entities with `entity_type = "LearnedPattern"` and `memory_type = Pattern` (never expires)
4. Pattern edges link to source compression groups via `episode_id` references
5. `Storage::store_pattern(pattern: &ExtractedPattern) -> Result<String>` persists a new pattern
6. `Storage::get_patterns() -> Result<Vec<ExtractedPattern>>` retrieves all learned patterns

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `ExtractedPattern`, `PatternEvidence`
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `store_pattern`, `get_patterns`, `get_patterns_for_entity`
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::extract_patterns` orchestration method
  - `crates/ctxgraph-core/src/pattern.rs` — new module with `PatternExtractor` (pure logic, no I/O)
- **New types/functions:**
  - `ExtractedPattern { id: String, name: String, description: String, evidence: Vec<PatternEvidence>, confidence: f64, created_at: DateTime<Utc> }`
  - `PatternEvidence { entity_types: Vec<String>, relation_triplet: Option<(String, String, String)>, source_groups: Vec<String>, occurrence_count: u32 }`
  - `PatternExtractor::extract(&self, compression_groups: &[CompressionGroup]) -> Vec<ExtractedPattern>`
- **Config changes:** none

### Test Plan
- Integration: compress 5 episode groups about Docker bugs, extract patterns, verify at least one pattern about Docker troubleshooting
- Integration: extracted pattern has `memory_type: Pattern` and no TTL (never expires)
- Integration: pattern links back to source compression groups
- Unit: `PatternExtractor` with 2 compression groups finds no patterns (threshold = 3)
- Unit: `PatternExtractor` with 4 groups sharing same entity pair finds pattern
- Integration: `get_patterns` returns previously extracted patterns

---

## D2: Skill creation and evolution

**Phase:** D
**Priority:** P1
**Effort:** L
**Depends on:** D1

### Description
Build on extracted patterns to create Skills — behavioral knowledge about what worked, what failed, and what the user preferred. A Skill is a higher-level abstraction than a pattern: it encodes an actionable rule. Skills are created when patterns show consistent success/failure (e.g. "When fixing Docker networking bugs, check DNS resolution first — this worked 4/5 times"). Skills can evolve: if new evidence contradicts a skill, it gets a `superseded_by` link to the updated skill. Skills have `memory_type: Pattern` and never expire.

### Acceptance Criteria
1. `Skill` struct with fields: `id`, `name`, `description`, `trigger_condition` (when to apply), `action` (what to do), `success_count`, `failure_count`, `confidence`, `superseded_by: Option<String>`
2. `SkillCreator` analyzes patterns with success/failure signals (edges with relation "fixed" = success, "deprecated" = failure) and creates skills
3. Skills stored in a new `skills` table with FTS5 index on description
4. `Storage::create_skill(skill: &Skill) -> Result<String>` and `Storage::list_skills() -> Result<Vec<Skill>>`
5. When new evidence contradicts a skill, `Storage::supersede_skill(old_id, new_id)` updates the old skill
6. Skills with `superseded_by` set are excluded from retrieval but kept for audit

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `Skill` struct
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 006 (skills table + FTS5)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `create_skill`, `list_skills`, `supersede_skill`, `search_skills`
  - `crates/ctxgraph-core/src/skill.rs` — new module with `SkillCreator`
- **New types/functions:**
  - `Skill { id: String, name: String, description: String, trigger_condition: String, action: String, success_count: u32, failure_count: u32, confidence: f64, superseded_by: Option<String>, created_at: DateTime<Utc>, entity_types: Vec<String> }`
  - `SkillCreator::create_from_patterns(patterns: &[ExtractedPattern]) -> Vec<Skill>`
  - `Storage::create_skill(&self, skill: &Skill) -> Result<String>`
  - `Storage::supersede_skill(&self, old_id: &str, new_id: &str) -> Result<()>`
- **Config changes:** none

### Test Plan
- Integration: extract patterns from 5 compression groups about Docker fixes, create skill, verify skill stored
- Integration: skill has `memory_type: Pattern` and no TTL
- Integration: list_skills returns active skills (not superseded ones)
- Integration: supersede a skill, verify old skill has `superseded_by` set, new skill is active
- Unit: `SkillCreator` with no success/failure signals creates no skills
- Integration: `search_skills` via FTS5 finds relevant skills

---

## D3: Cross-session skill persistence and sharing

**Phase:** D
**Priority:** P2
**Effort:** M
**Depends on:** D2

### Description
Skills persist across sessions (they're in SQLite, so this is automatic) and can optionally be shared across agents. Add a `scope` field to Skill: "private" (agent-only) or "shared" (available to all agents using the same graph DB). Shared skills are included in retrieval for any agent's `retrieve_for_context` call. Add a `skill_sources` table tracking which agents created/contributed to each skill. The MCP tool `learn` triggers pattern extraction and skill creation in one call.

### Acceptance Criteria
1. `Skill` struct has `scope: SkillScope` field with variants `Private` and `Shared`
2. `Storage::share_skill(id: &str) -> Result<()>` changes scope from Private to Shared
3. `retrieve_for_context` includes shared skills for any agent, private skills only for owning agent
4. `skill_sources` table tracks `(skill_id, agent_name, created_at)` for provenance
5. Skills survive across sessions (no action needed — SQLite persistence)
6. New `learn` MCP tool runs full pipeline: extract patterns from recent compressions, create/update skills

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `SkillScope` enum, `scope` field on `Skill`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 007 (skill_sources table, scope column)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `share_skill`, update `retrieve_for_context` to include skills
  - `crates/ctxgraph-mcp/src/tools.rs` — add `learn` tool handler and definition
- **New types/functions:**
  - `SkillScope` enum: `Private`, `Shared`
  - `Storage::share_skill(&self, id: &str) -> Result<()>`
  - `Storage::get_skills_for_agent(&self, agent_name: &str) -> Result<Vec<Skill>>`
  - `ToolContext::learn` MCP handler
- **Config changes:** none

### Test Plan
- Integration: create private skill for agent "coding", verify agent "coding" sees it, agent "finance" does not
- Integration: `share_skill` makes skill visible to all agents
- Integration: `skill_sources` tracks which agent created the skill
- Integration: new session (reopen DB) — skills still present
- Integration: MCP `learn` tool returns created/updated skills count
- Unit: skill with no scope defaults to Private

---

## D4: Learn CLI command and MCP tool

**Phase:** D
**Priority:** P2
**Effort:** S
**Depends on:** D1, D2, D3

### Description
Expose the learning pipeline via CLI and MCP. CLI gets `ctxgraph learn` subcommand that runs pattern extraction and skill creation, with options to scope output and show results. MCP gets `learn` tool (from D3) plus `list_skills` and `share_skill` tools. The CLI shows a summary of new patterns found and skills created/updated. Add skill display to `ctxgraph stats` output.

### Acceptance Criteria
1. CLI: `ctxgraph learn` runs full learning pipeline and reports new patterns and skills
2. CLI: `ctxgraph learn --dry-run` shows what would be learned without persisting
3. CLI: `ctxgraph learn --scope shared` creates skills as shared by default
4. MCP: `list_skills` tool returns all active (non-superseded) skills for the agent
5. MCP: `share_skill` tool with `{id: "..."}` changes skill scope to shared
6. CLI: `ctxgraph stats` output includes skill count, pattern count, and shared vs private breakdown

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-cli/src/commands/learn.rs` — new command module
  - `crates/ctxgraph-cli/src/commands/mod.rs` — register learn module
  - `crates/ctxgraph-cli/src/main.rs` — add `Learn` subcommand
  - `crates/ctxgraph-mcp/src/tools.rs` — add `list_skills`, `share_skill` tool definitions
  - `crates/ctxgraph-cli/src/commands/stats.rs` — extend to include skill/pattern counts
- **New types/functions:** `commands::learn::run(dry_run, scope)`, MCP tool handlers for `list_skills`, `share_skill`
- **Config changes:** none

### Test Plan
- Integration: `ctxgraph learn` on DB with compressed episodes creates patterns and skills
- Integration: `ctxgraph learn --dry-run` outputs "would create N patterns, M skills" without writing
- Integration: `ctxgraph learn --scope shared` creates skills with Shared scope
- Integration: MCP `list_skills` returns JSON array of active skills
- Integration: MCP `share_skill` with valid ID returns success
- Integration: `ctxgraph stats` shows "Skills: 5 (3 shared, 2 private)"

---

# Dependency Graph

```
A1 (TTL + memory_type fields)
├── A2 (decay_score)
│   └── A4 (budget retrieval) ── depends on A1, A2, A3
├── A3 (usage tracking)
│   ├── A4 (budget retrieval)
│   └── C2 (implicit renewal)
└── A5 (per-agent policies)
    └── A4 (budget retrieval)

B1 (compression pipeline) ── depends on A1, A3
├── B2 (relationship inheritance) ── depends on B1
├── B3 (compression triggers) ── depends on B1, A5
└── B4 (compress CLI/MCP) ── depends on B1, B3

C1 (contradiction detection) ── depends on A1, A2, A3
C2 (implicit renewal) ── depends on A1, A3
C3 (active re-verification) ── depends on A1, A2, A3
C4 (reverify CLI/MCP) ── depends on C1, C2, C3

D1 (pattern extraction) ── depends on B1, B2, A1
├── D2 (skill creation) ── depends on D1
│   └── D3 (cross-session sharing) ── depends on D2
└── D4 (learn CLI/MCP) ── depends on D1, D2, D3
```

# Effort Summary

| Phase | Stories | P0 | P1 | P2 | Total Effort |
|-------|---------|----|----|-----|-------------|
| A     | 5       | 3  | 2  | 0   | 3M + 2S      |
| B     | 4       | 1  | 3  | 0   | 1L + 2M + 1S |
| C     | 4       | 1  | 1  | 2   | 1L + 1M + 2S |
| D     | 4       | 0  | 2  | 2   | 2L + 1M + 1S |
| Total | 17      | 5  | 8  | 4   | 3L + 8M + 6S |

# Migration Plan

| Migration | Story | Changes |
|-----------|-------|---------|
| 003       | A1    | Add `memory_type`, `ttl_seconds` to entities + edges + episodes |
| 004       | A3    | Add `usage_count`, `last_recalled_at` to entities + edges |
| 005       | B1    | Add `compression_groups` table, `compression_id` to episodes |
| 006       | D2    | Add `skills` table + FTS5 index |
| 007       | D3    | Add `skill_sources` table, `scope` column to skills |
