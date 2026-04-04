# ctxgraph4agent — User Stories (Final)

> Approved after 3 rounds of review (GLM 5.1 designer + MiniMax M2.7 reviewer)
> Memory lifecycle: Store → TTL → Forget → Decay → Re-verify → Compress → Budget → Learn
>
> Changes from Round 1: A4 split into A4a/A4b/A4c, D1 split into D1a/D1b, A6 (TTL cleanup) added,
> A3 trimmed: renewal_count deferred to C2, touch_many/get_usage_stats/MemoryTable/indexes removed, effort M→S, B1 summary template→LLM + memory_type Pattern→Fact + LLM moved to Graph layer + fallback added, D1b description template→LLM + ExtractedPattern/PatternEvidence types removed + known simplification documented, B3 compression no longer runs every query, C2 uses renewal_count
> not usage_count, C1 uses entity_id matching with confidence threshold, C3 stale_threshold
> configurable, C4 update format defined, D2 success/failure relations configurable, D2 skill synthesis template→LLM + DraftSkill pipeline + memory_type: Pattern removed (skills not entities) + confidence formula + provenance renewal_count separated from C2 + LLM failure fallback + deferred provenance-entity linking/success_count live updates/provenance re-evaluation
> D3 simplified: skill_sources table→columns on skills table, persistence AC removed, skill budget integration defined (0.8 floor score), sharing irreversibility documented as POC,
> D4 pipeline defined (D1a→D1b→dedup→D2→D3), --limit caps skills not patterns, --format json added, --agent flag added, dedup/supersession check added, edge case tests added
> A5 review fixes: decisions_ttl default fixed to never (matches CLAUDE.md), compress_after=14d default added to AC2, max_episodes default 1000 added, fallback resolution test added, default_agent config key defined (AC8)

## Phase A: TTL + Decay (Foundation)

Stories A1-A6 establish the core memory lifecycle. Every subsequent phase depends on these fields and computations existing first.

---

## A1: Add ttl and memory_type fields to Entity and Edge

**Phase:** A
**Priority:** P0
**Effort:** M
**Depends on:** none

### Description
Add `memory_type` (Fact, Pattern, Experience, Preference, Decision) and `ttl` (Option<Duration>) fields to both Entity and Edge structs. These fields are persisted in SQLite via a new migration (003). Episodes also get `memory_type` to classify the source. The `memory_type` defaults based on `entity_type` mapping (e.g. Decision entity -> Decision memory_type, Component -> Fact) but can be overridden explicitly. If entity_type doesn't map to any known MemoryType, default to Fact.

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

### Migration 003 (idempotency fix)
Migration 003's UPDATE clause must use `WHERE ttl_seconds IS NULL` to avoid overwriting user-customized TTLs when migration is run multiple times:
```sql
UPDATE entities SET memory_type = 'Fact', ttl_seconds = ... WHERE ttl_seconds IS NULL;
UPDATE edges SET memory_type = 'Fact', ttl_seconds = ... WHERE ttl_seconds IS NULL;
```
This ensures idempotent re-runs — explicit TTLs set by users or code are preserved.

### Test Plan
- Unit: `MemoryType::default_for_entity_type("Decision")` returns `Decision`
- Unit: `MemoryType::default_for_entity_type("UnknownType")` returns `Fact` (fallback)
- Unit: `MemoryType::default_ttl(&Fact)` returns `Some(Duration::from_secs(90 * 86400))`
- Unit: `MemoryType::default_ttl(&Pattern)` returns `None`
- Integration: insert entity with no explicit ttl, read it back, verify ttl is 90 days for Fact
- Integration: migration 003 applied to existing DB, existing rows get `memory_type='Fact'` and `ttl_seconds` populated via UPDATE (only where IS NULL)
- Integration: migration 003 run twice — second run is a no-op (rows already have values)
- Integration: entity with `ttl=None` (Pattern) persists and reads back as `None`

---

## A2: Implement decay_score computation

**Phase:** A
**Priority:** P0
**Effort:** M
**Depends on:** A1

### Description
Implement the `decay_score` function that computes freshness at query time (not stored). The function takes a node's memory_type, ttl, base confidence, age (now - created_at), and returns a score in [0.0, 1.0]. Three decay functions: exponential for facts (half-life = TTL/2), constant 1.0 for patterns, and linear drop to 0.0 at TTL for experiences. Preferences and decisions use exponential like facts. If ttl is None (patterns), decay_score always returns the base confidence.

Explicit formulas (documented in doc comments):
- Exponential: `base_confidence * exp(-λ * age)` where `λ = ln(2) / half_life` and `half_life = ttl / 2`
- Linear: `base_confidence * max(0.0, 1.0 - (age / ttl))`
- Constant: `base_confidence`

### Acceptance Criteria
1. `decay_score` function computes `base_confidence * decay_function(age, memory_type, ttl)` returning f64 in [0.0, 1.0]
2. For Fact type: exponential decay with half-life = ttl/2 — a fact at age=0 scores 1.0, at age=ttl/2 scores 0.5, at age=ttl scores 0.25
3. For Pattern type: constant decay — always returns `base_confidence` regardless of age
4. For Experience type: linear decay — at age=0 scores 1.0, at age=ttl scores 0.0, linearly interpolated
5. For Preference and Decision: exponential decay same as Fact
6. If age > ttl (expired), the score is 0.0 for all types except Pattern (which never expires)
7. Edge case: if `ttl = 0`, returns 0.0 immediately

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `pub fn decay_score(&self, base_confidence: f64, created_at: DateTime<Utc>, ttl: Option<Duration>) -> f64` on `MemoryType`
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
- Unit: `Fact.decay_score(1.0, now)` with ttl=0 returns 0.0 (edge case)
- Property: all decay functions return values in [0.0, 1.0]

---

## A3: Add usage_count and last_recalled_at tracking

**Phase:** A
**Priority:** P1
**Effort:** S
**Depends on:** A1 (A4b depends on `usage_count`; A6 depends on both fields)

### Description
Add `usage_count: u32` and `last_recalled_at: Option<DateTime<Utc>>` to Entity and Edge. These two fields serve the two-tier memory lifecycle philosophy:

- **`usage_count`** — how often a memory is consumed in retrieval results. Two consumers: (1) A4b scoring bonus rewards frequently-recalled memories, (2) A6 6-month cleanup uses high usage_count as an automated "keep" signal (no LLM needed — "renewed recently OR high usage_count → keep").
- **`last_recalled_at`** — timestamp of last recall. Feeds "renewed recently?" check at 6-month cleanup boundary.

A new `touch_entity`/`touch_edge` method increments usage_count and sets last_recalled_at to now. These fire automatically when memories are consumed by the agent — no manual polling, no dashboard.

**Deferred:** `renewal_count` (tracks TTL renewals) is deferred to C2 where it's consumed. Adding it now with no Phase A consumer is premature. It will be a single column in a later migration.

### Acceptance Criteria
1. `Entity` and `Edge` structs have `usage_count: u32` (default 0) and `last_recalled_at: Option<DateTime<Utc>>` (default None)
2. Migration 004 adds `usage_count INTEGER NOT NULL DEFAULT 0` and `last_recalled_at TEXT` columns to `entities` and `edges` tables
3. `Storage::touch_entity(id: &str)` increments usage_count and sets last_recalled_at to Utc::now() in a single UPDATE
4. `Storage::touch_edge(id: &str)` same behavior for edges
5. All read paths (get_entity, list_entities, get_edge, etc.) populate the new fields

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add fields to `Entity` and `Edge`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 004
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `touch_entity`, `touch_edge`; update all read paths
- **New types/functions:**
  - `Storage::touch_entity(&self, id: &str) -> Result<()>`
  - `Storage::touch_edge(&self, id: &str) -> Result<()>`
- **Indexes:** none (deferred until query patterns and data volumes justify them)
- **Config changes:** none

### Test Plan
- Integration: insert entity, call `touch_entity` 3 times, read entity back, verify `usage_count == 3` and `last_recalled_at` is set
- Integration: call `touch_entity`, verify `last_recalled_at` is set to recent timestamp
- Integration: migration 004 applied to existing DB, existing rows get `usage_count=0`, `last_recalled_at=NULL`
- Unit: `touch_entity` on nonexistent id returns error

---

## A4a: FTS5 + graph candidate retrieval

**Phase:** A
**Priority:** P0
**Effort:** M
**Depends on:** A1, A3

### Description
Implement the candidate retrieval step that produces the initial set of memories to be ranked. This is the first of three stories that replace the original A4. Given a query string, retrieve candidates using two strategies: (1) FTS5 full-text search over entity names, edge labels, and episode content, returning top-N results ranked by BM25; (2) graph traversal from entities mentioned in the query — follow edges to 1-hop neighbors. Results from both strategies are deduplicated by (entity_id or edge_id) and returned as a `Vec<RetrievalCandidate>`. Patterns are retrieved only if they match the query (not all patterns in DB) — limited to a configurable `max_patterns_included` cap (default 50).

### Acceptance Criteria
1. `Storage::retrieve_candidates(query: &str, limit: usize, max_patterns: usize) -> Result<Vec<RetrievalCandidate>>` returns deduplicated candidates
2. FTS5 query searches entity names, edge labels/relation types, and episode content with BM25 ranking
3. Graph traversal follows edges from query-matched entities to 1-hop neighbors
4. Duplicate candidates (same entity or edge appearing from both strategies) are merged — keep higher BM25 score
5. Patterns are included only if they match the query via FTS5, up to `max_patterns_included` cap
6. Returns `Vec<RetrievalCandidate>` containing entity_id, edge_id, content preview, fts_score, memory_type, and metadata needed for scoring

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `RetrievalCandidate` struct
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `retrieve_candidates` method
- **New types/functions:**
  - `RetrievalCandidate { entity_id: Option<String>, edge_id: Option<String>, content: String, fts_score: f64, memory_type: MemoryType, created_at: DateTime<Utc>, ttl: Option<Duration>, base_confidence: f64, usage_count: u32 }`
  - `Storage::retrieve_candidates(&self, query: &str, limit: usize, max_patterns: usize) -> Result<Vec<RetrievalCandidate>>`
- **Config changes:** none (max_patterns set in MemoryPolicyConfig via A5)

### Test Plan
- Integration: insert 50 entities, query returns candidates sorted by relevance
- Integration: query matching 3 entities with shared edges returns both entities and edges as candidates
- Integration: deduplication — same entity from FTS5 and graph traversal appears once with higher score; graph-only candidates (no FTS match) use a default relevance score of 0.3
- Integration: pattern not matching query is not returned
- Integration: query returning no results returns empty vec (not error)
- Unit: max_patterns_included=0 returns no patterns even if they match

---

## A4b: Scoring and ranking with decay

**Phase:** A
**Priority:** P0
**Effort:** M
**Depends on:** A1, A2, A3, A4a

### Description
Implement the scoring and ranking step that computes a composite score for each retrieval candidate. This is the second of three stories that replace the original A4. The composite score uses decay_score (A2) for freshness, FTS5 BM25 for relevance, and usage_count (A3) for a recency bonus. The scoring bonus uses `usage_count` (how often recalled) — renewal_count is a Phase C concern (C2), not part of scoring. After scoring, candidates are sorted descending by composite score. Patterns always get a minimum score of 0.5 (floor) to ensure they surface unless budget is truly exhausted.

### Acceptance Criteria
1. `score_candidate(candidate: &RetrievalCandidate) -> f64` computes composite score
2. Composite score = `decay_score * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count))` (range 0.0 to ~1.46 — intentional: rewards frequently-recalled high-confidence memories) — usage_count is the recall frequency, NOT renewal_count
3. `normalized_fts_score` clamps BM25 to [0.0, 1.0] range for stable multiplication
4. Patterns (memory_type=Pattern) get `max(score, 0.5)` — floor of 0.5 to ensure visibility
5. Expired memories (decay_score = 0.0) get score = 0.0 and are filtered out before ranking
6. Returns `Vec<ScoredCandidate>` sorted descending by composite score

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `ScoredCandidate` struct, `score_candidate` function
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::rank_candidates` method
- **New types/functions:**
  - `ScoredCandidate { candidate: RetrievalCandidate, composite_score: f64 }`
  - `score_candidate(candidate: &RetrievalCandidate) -> f64`
  - `Graph::rank_candidates(&self, candidates: Vec<RetrievalCandidate>) -> Vec<ScoredCandidate>`
- **Config changes:** none

### Test Plan
- Unit: fresh fact with high FTS and usage_count=10 scores higher than stale fact with high FTS and usage_count=0
- Unit: pattern with low FTS score still gets at least 0.5 composite score
- Unit: expired memory (age > ttl) gets score 0.0
- Unit: usage_count=0 gives bonus factor of 1.0 (no bonus)
- Unit: usage_count=100 gives bonus factor of ~1.46 (diminishing returns from ln)
- Integration: 50 candidates ranked, top result has highest composite score
- Property: composite score is always in [0.0, ~1.5] range

---

## A4c: Budget enforcement and token counting

**Phase:** A
**Priority:** P0
**Effort:** S
**Depends on:** A4a, A4b

### Description
Implement the budget enforcement step that greedily fills a token budget from ranked candidates. This is the third of three stories that replace the original A4. Given a sorted list of `ScoredCandidate` (from A4b), greedily add candidates until the budget is exhausted. Token counting uses `text.len() / 4` as a ceiling estimate (acknowledged to be imprecise — documented as such). Returns `Vec<RankedMemory>` plus `token_budget_spent` so callers know actual consumption. A new `retrieve_for_context` method on Storage orchestrates all three steps (A4a -> A4b -> A4c) into a single call.

### Acceptance Criteria
1. `enforce_budget(candidates: Vec<ScoredCandidate>, budget_tokens: usize) -> (Vec<RankedMemory>, usize)` returns memories within budget plus tokens spent
2. Greedy selection: add highest-scored candidates first, skip if adding would exceed budget
3. Total token count of returned memories does not exceed budget (default 20,000 tokens)
4. Token estimation uses `text.len() / 4` — documented as ceiling estimate, not exact
5. If a single memory exceeds the budget, it is skipped (not returned)
6. `Storage::retrieve_for_context(query: &str, agent_name: &str, budget_tokens: usize)` orchestrates A4a + A4b + A4c in one call — the method looks up the agent's policy internally using agent_name
7. `enforce_budget` returns `(Vec<RankedMemory>, usize)` where the second element is the total tokens spent

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `RankedMemory` struct, `AgentPolicy` struct, `estimate_tokens` function
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `retrieve_for_context` orchestration method
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::retrieve_for_context` passthrough
- **New types/functions:**
  - `RankedMemory { memory_type: MemoryType, content: String, score: f64, entity_id: Option<String>, edge_id: Option<String>, tokens: usize }`
  - `AgentPolicy { memory_budget_tokens: usize, agent_name: String, max_patterns_included: usize }`
  - `AgentPolicy::default() -> AgentPolicy` (budget = 20_000, max_patterns = 50)
  - `estimate_tokens(text: &str) -> usize` (text.len() / 4, documented as ceiling estimate)
  - `enforce_budget(candidates: Vec<ScoredCandidate>, budget_tokens: usize) -> (Vec<RankedMemory>, usize)`
- **Config changes:** none (policy loaded from config in A5)

### Test Plan
- Unit: `estimate_tokens("hello world")` returns ~3
- Unit: `enforce_budget` with 50 candidates and budget=100 returns 1-2 small memories
- Unit: single memory larger than budget is skipped
- Integration: insert 50 entities, `retrieve_for_context` returns <= 20k tokens worth
- Integration: patterns within max_patterns_included cap are included
- Integration: if budget is 0, returns empty vec
- Property: sum of estimate_tokens for all results <= budget_tokens

---

## A5: Per-agent memory policies via ctxgraph.toml [policies] section

**Phase:** A
**Priority:** P1
**Effort:** M
**Depends on:** A1, A4c

### Description
Extend `ctxgraph.toml` with a `[policies.<agent_name>]` section that configures TTL, budget, and compression settings per agent. Each policy specifies TTLs per memory_type, memory_budget_tokens, compress_after duration, max_episodes, max_patterns_included, and stale_threshold. The `MemoryPolicyConfig` struct is loaded alongside the existing `ExtractionSchema`. A `set_policy` MCP tool allows runtime policy changes (session override only — not persisted to file; persisted policies must be written to ctxgraph.toml). The `retrieve_for_context` method (A4c) uses the active agent's policy.

### Acceptance Criteria
1. `ctxgraph.toml` supports `[policies.<name>]` with keys: `facts_ttl`, `experiences_ttl`, `patterns_ttl` (always "never"), `preferences_ttl`, `decisions_ttl`, `memory_budget_tokens`, `compress_after`, `max_episodes` (default 1000), `max_patterns_included` (default 50), `stale_threshold` (default 0.3), `provenance_ttl_days` (default 180), `context_ttl_days` (default 90)
2. `MemoryPolicyConfig` struct deserializes from TOML with defaults matching the assistant policy from CLAUDE.md (facts=90d, experiences=14d, patterns=never, preferences=30d, decisions=never, budget=20000, compress_after=14d, provenance_ttl_days=180, context_ttl_days=90)
3. `ctxgraph.toml.example` updated with the policies section
4. New MCP tool `set_policy` allows changing TTL/budget at runtime for the active agent (session override, not persisted)
5. `Graph::init` loads policies from `.ctxgraph/ctxgraph.toml` if present, falls back to defaults
6. Invalid policy values (e.g. negative TTL, budget < 1000) produce a clear error
7. Validation warning if `compress_after < 7` days (logged but not blocking)
8. Config section `[policies]` supports optional `default_agent = "assistant"` key — used by CLI commands (D4's `--agent` flag default) and MCP session context

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `MemoryPolicyConfig` struct, extend TOML parsing
  - `crates/ctxgraph-core/src/graph.rs` — load policy config in `init` and `open`, store as field on `Graph`
  - `crates/ctxgraph-mcp/src/tools.rs` — add `set_policy` tool handler and tool definition
  - `ctxgraph.toml.example` — add policies section
- **New types/functions:** `MemoryPolicyConfig`, `MemoryPolicyConfig::load(path: &Path)`, `MemoryPolicyConfig::default()`, `MemoryPolicyConfig::for_agent(&self, agent_name: &str) -> Option<&AgentPolicy>`
- **Config changes:** new `[policies.<agent>]` section in ctxgraph.toml

### Test Plan
- Unit: parse a TOML string with `[policies.programming]` section, verify TTL values
- Unit: `MemoryPolicyConfig::default()` returns assistant policy defaults
- Unit: invalid TTL string like "banana" returns `CtxGraphError::Schema(...)`
- Unit: policy with budget < 1000 returns validation error
- Unit: compress_after = 3 days logs warning but still parses
- Unit: `for_agent` with unknown agent name returns default policy (fallback behavior)
- Integration: `Graph::init` creates DB and loads default policy
- Integration: MCP `set_policy` tool changes budget, subsequent retrievals use new budget (session-scoped)
- Integration: MCP `set_policy` does not write to ctxgraph.toml file

---

## A6: TTL enforcement and cleanup

**Phase:** A
**Priority:** P0
**Effort:** L
**Depends on:** A1, A2, A3

### Description
Implement TTL enforcement that deletes or archives nodes where `decay_score = 0` for longer than a configurable `grace_period` (default 7 days). This is critical to the "stays within budget" thesis — without cleanup, expired data accumulates indefinitely. Cleanup runs lazily at query time (checked every N queries, not every query) and can also be triggered manually via CLI/MCP. After grace_period expires: Facts and Experiences are deleted (purged from DB), Preferences and Decisions are archived (moved to an `archived_entities`/`archived_edges` table for audit). Patterns are never cleaned up (they don't decay). A `last_cleanup_at` timestamp and `cleanup_in_progress` flag prevent redundant or concurrent cleanup runs.

### Acceptance Criteria
1. `Storage::cleanup_expired(policy: &AgentPolicy) -> Result<CleanupResult>` identifies and processes expired memories
2. Grace period: memories with `decay_score = 0` for > `grace_period` (default 7 days) are eligible for cleanup
3. Facts and Experiences are purged (DELETE from entities/edges + CASCADE to episode_entities, edges)
4. Preferences and Decisions are archived to `archived_entities`/`archived_edges` tables before deletion
5. Patterns are never cleaned up regardless of age
6. `last_cleanup_at: Option<DateTime<Utc>>` stored in a `system_metadata` table; cleanup skipped if last run was within `cleanup_interval` (default 100 queries)
7. `cleanup_in_progress: bool` flag prevents concurrent cleanup (checked and set atomically)
8. `CleanupResult` struct reports `{ entities_deleted: usize, entities_archived: usize, edges_deleted: usize, edges_archived: usize }`

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `CleanupResult`, `CleanupStrategy` enum (Purge, Archive)
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 005 (archived_entities, archived_edges, system_metadata tables)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `cleanup_expired`, `archive_entity`, `archive_edge`, `should_cleanup`, `mark_cleanup_done`
  - `crates/ctxgraph-core/src/graph.rs` — integrate lazy cleanup into retrieval path (every N queries)
- **New types/functions:**
  - `CleanupResult { entities_deleted: usize, entities_archived: usize, edges_deleted: usize, edges_archived: usize }`
  - `Storage::cleanup_expired(&self, policy: &AgentPolicy) -> Result<CleanupResult>`
  - `Storage::should_cleanup(&self, interval: u32) -> Result<bool>` (checks last_cleanup_at)
  - `Storage::mark_cleanup_done(&self) -> Result<()>`
  - `Storage::get_cleanup_stats(&self) -> Result<CleanupStats>`
- **Indexes:** Add index on `created_at` for decay-based cleanup queries; add index on `(memory_type, created_at)` for efficient expired record scanning
- **Config changes:** `grace_period` (default "7d") and `cleanup_interval` (default 100) in `[policies.<agent>]`

### Test Plan
- Integration: insert entity 100 days ago (ttl=90d), run cleanup after grace_period, verify entity deleted
- Integration: insert preference 100 days ago (ttl=30d), run cleanup, verify preference archived (exists in archived_entities)
- Integration: pattern 365 days old is NOT deleted by cleanup
- Integration: cleanup skipped if last_cleanup_at is within cleanup_interval
- Integration: `cleanup_in_progress` flag prevents double-run
- Integration: `ctxgraph cleanup` CLI runs cleanup and reports result
- Integration: CleanupResult counts match actual DB changes
- Unit: cleanup on empty DB returns zero counts

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
Implement a compression pipeline that batches old episodes into a single summary node. Given a set of episode IDs, the pipeline creates a new "compressed episode" with a **content-quality summary**, marks the source episodes with `compression_id` linking them to the summary, and sets their decay to accelerated.

The summary is generated via LLM (not template). Template-based summaries produce metadata, not meaning — *"In March, Docker, auth, and PostgreSQL were involved in 14 episodes"* is useless for retrieval. The compressed node must preserve the **value** of the source episodes: what happened, what was decided, what was learned. CLAUDE.md sets the bar: *"Week of March 24: focused on auth migration, resolved 3 bugs"* — a concise, actionable summary an agent can actually use.

LLM call frequency is low (batch compression runs per trigger interval, not per query), so the cost is negligible compared to the retrieval value of a good summary.

**Architecture:** The LLM call lives in the `Graph` layer (orchestration), not in `Storage` (persistence). Storage only handles insert/update operations. This keeps Storage testable without mocking LLM clients and aligns with "SQLite first, zero deps" — Storage stays a pure SQLite wrapper.

**Memory type:** Compressed summaries use `memory_type: Fact` (not Pattern). A compressed summary is a factual record of what happened during a period — it should decay and be re-verified like any other fact. Using Pattern would make summaries never expire (Pattern = learned behaviors, keep forever), which breaks the TTL/Decay lifecycle after 6 months of use. Facts have 90d TTL by default, configurable via per-agent policy (A5).

**Fallback:** If the LLM call fails (timeout, unavailable, bad response), compression is skipped for this batch. Source episodes remain uncompressed and will be retried on the next trigger cycle. No silent degradation to template — either we get a quality summary or we wait.

### Acceptance Criteria
1. `Graph::compress_episodes(episode_ids: &[String]) -> Result<String>` orchestrates: load episodes → generate LLM summary → store compressed episode → link source episodes → returns compressed episode ID
2. `Storage::compress_episodes(&self, episode_ids: &[String], summary: &str) -> Result<String>` creates a new episode with `memory_type: Fact`, the LLM-generated summary as content, and returns its ID
3. Summary generation prompt (defined in `Graph` layer): "Summarize these episodes into a concise 2-3 sentence summary preserving key decisions, outcomes, and patterns. Focus on what happened, not who was involved." — source episode contents concatenated and truncated to token budget before sending
4. Each source episode gets `compression_id` set to the new summary episode's ID via UPDATE
5. `compression_id: Option<String>` field added to `Episode` struct (persisted in migration 006)
6. The compressed summary episode has all entities from source episodes merged as `episode_entities` links
7. Compressed episodes are queryable via normal search; source episodes remain until decayed
8. Compressing empty episode_ids returns error
9. Summary is stored as the episode `content` field — no separate `compressed_at` field (use `recorded_at`)
10. LLM call failure returns error but does not modify source episodes; next trigger cycle retries

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `compression_id: Option<String>` to `Episode`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 006 (`compression_id TEXT` on episodes)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `compress_episodes`, update episode read/write paths
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::compress_episodes` (orchestration + LLM call), `Graph::generate_compression_summary`
- **New types/functions:**
  - `Graph::compress_episodes(&self, episode_ids: &[String]) -> Result<String>` — full pipeline
  - `Graph::generate_compression_summary(&self, episodes: &[Episode]) -> Result<String>` — LLM call with prompt
  - `Storage::compress_episodes(&self, episode_ids: &[String], summary: &str) -> Result<String>` — persistence only
  - `Storage::list_uncompressed_episodes(&self, before: DateTime<Utc>) -> Result<Vec<Episode>>`
- **Config changes:** LLM model for compression uses the Graph's default model. Per-agent compression model override can be added in A5 if needed.

### Test Plan
- Integration: insert 5 episodes about Docker debugging, compress them, verify summary is a human-readable 2-3 sentence summary mentioning Docker and key outcomes (not just entity names)
- Integration: source episodes have `compression_id` set to new summary ID
- Integration: `list_uncompressed_episodes` returns only ungrouped episodes before the given date
- Integration: compressed episode has merged entity links from all source episodes
- Integration: compressed episode has `memory_type: Fact` (not Pattern)
- Integration: compressed episode is queryable via normal search and subject to normal decay
- Unit: compressing empty episode_ids returns error
- Unit: LLM call failure (mocked) returns error, source episodes untouched
- Unit: summary generation produces text (not empty, not just entity names)

---

## B2: Relationship inheritance from compressed nodes

**Phase:** B
**Priority:** P1
**Effort:** M
**Depends on:** B1

### Description
When episodes are compressed into a summary, the edges (relationships) from the source episodes should be inherited by the summary node. Duplicate edges (same source_id + target_id + relation) are merged into a single edge with accumulated confidence. The inherited edges retain their original memory_type but get a metadata flag `{"inherited_from": "compression_id", "source_edges": ["id1", "id2"]}`. Edges that become redundant (both endpoints now linked to the compressed summary) are invalidated. Inherited edges get new IDs (cleaner for deletion/TTL purposes). Metadata is merged by unioning JSON objects and concatenating arrays.

### Acceptance Criteria
1. `compress_episodes` also copies/merges edges from source episodes to the summary episode
2. Duplicate edges (same source + target + relation) are merged: confidence = max of source confidences, metadata lists source edge IDs
3. Inherited edges get metadata `{"inherited_from": "<compression_id>", "source_edges": ["id1", "id2"]}`
4. `Storage::get_edges_for_entity` on the compressed summary returns all inherited edges
5. Original edges from source episodes are NOT deleted (they decay naturally per their TTL)
6. Edge merging is idempotent — running compression on the same group twice is safe
7. Inherited edges get new IDs (not preserving original edge IDs)
8. Uniqueness constraint on `(source_id, target_id, relation)` in `edges` prevents duplicate inherited edge insertions

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — extend `compress_episodes` to handle edge inheritance, add `merge_edges` helper
- **New types/functions:** `Storage::merge_edges_for_compression(&self, compression_id: &str, episode_ids: &[String]) -> Result<usize>`
- **Config changes:** none

### Test Plan
- Integration: compress 3 episodes with overlapping entities, verify summary has merged edges (deduplicated)
- Integration: merged edge has max confidence of source edges
- Integration: merged edge metadata contains `inherited_from` field with source edge IDs
- Integration: original edges still exist and are queryable
- Unit: compressing episodes with no edges produces summary with no inherited edges
- Unit: merging edges with same (source, target, relation) from two compression groups is safe (uniqueness constraint)

---

## B3: Compression triggers (lazy interval-based)

**Phase:** B
**Priority:** P1
**Effort:** M
**Depends on:** B1, A5, A6

### Description
Add automatic compression triggers that run lazily every N queries (configurable, default 50), NOT every query. This is a critical performance fix from the review — running compression at every retrieve_for_context call adds unpredictable latency. Two trigger types: time-based (compress episodes older than `compress_after` days) and size-based (compress when episode count exceeds `max_episodes`). The trigger checks use a `last_compression_at` timestamp to avoid re-checking if compression ran recently. A `compression_in_progress` flag prevents concurrent compression runs (important for async or multi-threaded access). Triggers respect per-agent policy settings from A5. Compression is also available at init time and write time (after add_episode) as additional trigger points.

### Acceptance Criteria
1. `CompressionTrigger` struct evaluates whether compression should run given current state, policy, and query count since last check
2. `last_compression_at: Option<DateTime<Utc>>` stored in `system_metadata` table (shared with A6 cleanup)
3. `compression_in_progress: bool` flag checked and set atomically before compression begins
4. Query-interval trigger: compression check runs every `compression_check_interval` queries (default 50), NOT every query
5. Time-based trigger: if any ungrouped episodes are older than `compress_after` days, group them by source and compress (oldest batch first)
6. Size-based trigger: if total ungrouped episode count exceeds `max_episodes`, compress oldest batch until under limit
7. Compression grouping strategy: `BySource` — group episodes by their `source` field, compress each group separately
8. `Storage::get_compressible_episodes(before: DateTime<Utc>) -> Result<Vec<Episode>>` finds candidates
9. Compression respects memory_type: only experiences and decisions are compressible; facts and patterns are not

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `CompressionTrigger` struct, `CompressionStrategy` enum
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `get_compressible_episodes`, update `retrieve_for_context` to check trigger interval
  - `crates/ctxgraph-core/src/graph.rs` — integrate interval-based trigger checks into retrieval path
- **New types/functions:**
  - `CompressionTrigger { compress_after: Duration, max_episodes: usize, check_interval: u32, strategy: CompressionStrategy }`
  - `CompressionStrategy` enum: `BySource`, `OldestFirst`
  - `CompressionTrigger::should_check(&self, queries_since_last: u32) -> bool` (only check every N queries)
  - `CompressionTrigger::should_compress(&self, episode_count: usize, oldest_episode_age: Duration) -> bool`
  - `Graph::auto_compress(&self, policy: &MemoryPolicyConfig) -> Result<Option<CompressionResult>>`
  - `Graph::increment_query_counter(&self) -> u32`
- **Config changes:** `compress_after`, `max_episodes`, `compression_check_interval` (default 50) in `[policies.<agent>]`

### Test Plan
- Unit: `CompressionTrigger::should_check` with 49 queries and interval=50 returns false; with 50 returns true
- Unit: `CompressionTrigger::should_compress` with 100 episodes and max=50 returns true
- Unit: `CompressionTrigger::should_compress` with 10 episodes and max=50 returns false
- Unit: time-based trigger fires for episodes older than compress_after
- Integration: insert 60 episodes, trigger compresses oldest 10 to get under max_episodes=50
- Integration: experiences are compressed but facts are not
- Integration: trigger is a no-op when no episodes meet criteria
- Integration: `compression_in_progress` flag prevents double-run
- Integration: compression does NOT run on every retrieve_for_context call (only every 50th)

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
7. CLI: `ctxgraph compress --quiet` suppresses output for scripting
8. CLI: `ctxgraph compress --format json` outputs machine-readable JSON

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-cli/src/commands/compress.rs` — new command module
  - `crates/ctxgraph-cli/src/commands/mod.rs` — register compress module
  - `crates/ctxgraph-cli/src/main.rs` — add `Compress` subcommand variant
  - `crates/ctxgraph-mcp/src/tools.rs` — add `compress` tool handler and definition
- **New types/functions:** `commands::compress::run(dry_run, source, force, quiet, format)`, MCP tool handler `ToolContext::compress`
- **Config changes:** none

### Test Plan
- Integration: `ctxgraph compress --dry-run` on empty DB shows "nothing to compress"
- Integration: `ctxgraph compress` on DB with 60 episodes compresses and reports count
- Integration: MCP `compress` tool returns valid JSON with compression_id
- Integration: `ctxgraph compress --source nonexistent` returns "no episodes found for source"
- Integration: `ctxgraph compress --quiet` produces no stdout output on success
- Integration: `ctxgraph compress --format json` produces valid JSON output

---

## Phase C: Re-verify (Quality Maintenance)

Stories C1-C4 ensure memories stay accurate through contradiction detection and TTL renewal.

---

## C1: Passive re-verification (detect contradictions at write time)

**Phase:** C
**Priority:** P0
**Effort:** L
**Depends on:** A1, A3

### Description
When a new episode is ingested, the system checks existing facts for contradictions. If a new fact conflicts with a stored one (same entity + same relation type but different target entity or fact value), the old edge is invalidated and the new one takes precedence. **Review fix:** contradiction detection uses `entity_id` (not entity_name) as the primary matching key when entity_id is available and stable across episodes — entity_name is used as fallback only when entity_id is absent. A confidence threshold is applied: contradictions are only flagged if the existing edge has confidence >= `contradiction_threshold` (default 0.2) — low-confidence edges are simply replaced without flagging. Entity names are normalized (lowercase, trimmed) for fuzzy matching. Newer facts always win over older facts (time-based precedence).

### Acceptance Criteria
1. `Storage::check_contradictions(&self, new_edges: &[Edge]) -> Result<Vec<Contradiction>>` scans for conflicts using entity_id as primary key
2. A contradiction is detected when: same source entity_id (or entity_name as fallback) + same relation type, but different target entity or fact value
3. Contradiction only flagged if existing edge confidence >= `contradiction_threshold` (default 0.2)
4. When contradiction found, the old edge is invalidated (`valid_until = now`) and metadata updated
5. `Contradiction` struct records `{old_edge_id, new_edge_id, entity_id: Option<String>, entity_name: String, relation, old_value, new_value, existing_confidence: f64}`
6. Contradiction invalidation is called automatically during `Graph::add_episode` after extraction
7. Invalidated edges are no longer returned by `get_current_edges_for_entity` but remain in history
8. Entity name normalization: lowercase + trim whitespace before matching

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `Contradiction` struct
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `check_contradictions`, `invalidate_contradicted`
  - `crates/ctxgraph-core/src/graph.rs` — call contradiction check in `add_episode`
- **New types/functions:**
  - `Contradiction { old_edge_id: String, new_edge_id: String, entity_id: Option<String>, entity_name: String, relation: String, old_value: String, new_value: String, existing_confidence: f64 }`
  - `Storage::check_contradictions(&self, edges: &[Edge]) -> Result<Vec<Contradiction>>`
  - `Graph::add_episode_with_contradiction_check(&self, episode: Episode) -> Result<(EpisodeResult, Vec<Contradiction>)>`
  - `normalize_entity_name(name: &str) -> String` (lowercase + trim)
- **Config changes:** `contradiction_threshold: f64` (default 0.2) in `[policies.<agent>]`

### Test Plan
- Integration: insert "Alice chose PostgreSQL", then insert "Alice chose MySQL", verify first edge invalidated
- Integration: invalidated edge has `valid_until` set and `contradicted_by` in metadata
- Integration: insert "Alice chose PostgreSQL" twice — no contradiction (same fact)
- Integration: `get_current_edges_for_entity` returns only the newer edge
- Integration: existing edge with confidence=0.1 is replaced silently (no contradiction flagged, below threshold)
- Unit: contradiction check on empty graph returns empty vec
- Unit: `normalize_entity_name("  Alice  ")` returns `"alice"`
- Integration: entity_id-based matching used when both episodes share the same entity_id

---

## C2: Implicit TTL renewal (recalled and used -> auto-renew)

**Phase:** C
**Priority:** P1
**Effort:** M
**Depends on:** A1, A3

### Description
When a memory is recalled via `retrieve_for_context` and its content is actually used (appears in the context sent to the agent), its TTL is implicitly renewed. Renewal resets the effective age to 0 for decay calculation purposes by updating `created_at` to `Utc::now()`. This is gated by a `max_renewals` policy setting (default 5). **Critical fix:** renewal counting uses `renewal_count` (added in this story's migration 009, NOT `usage_count` from A3) — `renewal_count` only increments when renewal actually occurs, while `usage_count` tracks general recall frequency for scoring. Only Facts and Preferences are eligible for renewal; Experiences are not (they decay linearly and are meant to be forgotten).

### Acceptance Criteria
1. `Storage::renew_memory(id: &str, memory_type: MemoryType) -> Result<bool>` updates `created_at` to now and increments `renewal_count` if renewal is allowed
2. Renewal only applies to `Fact` and `Preference` memory types (not Experience, Pattern, or Decision)
3. Renewal count tracked via `renewal_count` (separate from `usage_count`) — if `renewal_count >= max_renewals`, renewal is denied
4. `MemoryPolicyConfig` has `max_renewals: usize` field (default 5)
5. Renewal returns false (no-op) if memory is already expired (decay_score = 0.0)
6. `retrieve_for_context` automatically calls `renew_memory` for each returned memory (only Facts and Preferences)
7. Renewal only fires for memories that appear in the final returned results (within budget), not all candidates

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `max_renewals` to `MemoryPolicyConfig` / `AgentPolicy`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 009 (`renewal_count INTEGER NOT NULL DEFAULT 0` on entities + edges)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `renew_memory`, integrate into `retrieve_for_context`
- **New types/functions:** `Storage::renew_memory(&self, id: &str, memory_type: MemoryType) -> Result<bool>`
- **Config changes:** `max_renewals = 5` in `[policies.<agent>]`

### Test Plan
- Integration: insert fact, recall it, verify `created_at` updated to recent time AND `renewal_count == 1`
- Integration: recall same fact 6 times (max_renewals=5), 6th recall does not renew (renewal_count stays at 5)
- Integration: verify `usage_count` continues incrementing past 5 even when renewal is denied (separate counters)
- Integration: recall experience — no renewal happens (not eligible type)
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
Implement a `get_stale_memories` method that surfaces memories approaching TTL expiration for active re-verification. **Review fix:** the stale threshold is configurable per-agent via `stale_threshold` in `MemoryPolicyConfig` (default 0.3), not hardcoded. The agent (or user via CLI/MCP) can review these and choose to renew, update, or let them expire. Stale memories are returned with their current content and a suggested action based on memory_type. This is opt-in — only called explicitly, not automatically. Results are paginated for agents with many stale memories.

### Acceptance Criteria
1. `Storage::get_stale_memories(threshold: f64, limit: usize, offset: usize) -> Result<Vec<StaleMemory>>` returns memories with decay_score < threshold, paginated
2. `StaleMemory` struct includes the memory content, entity/edge info, decay_score, age, and suggested action (renew/update/expire)
3. Suggested action: Facts -> "verify or update", Preferences -> "confirm with user", Experiences -> "let expire", Patterns -> never stale
4. `stale_threshold` defaults to 0.3 but is configurable per-agent in `MemoryPolicyConfig`
5. MCP tool `reverify` returns stale memories with a prompt for the agent to act on
6. CLI: `ctxgraph reverify list` shows stale memories in a human-readable table
7. CLI: `ctxgraph reverify renew <id>` explicitly renews a memory, bypassing max_renewals

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `StaleMemory`, `StaleAction` enum
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `get_stale_memories` (queries entities + edges with decay check)
  - `crates/ctxgraph-mcp/src/tools.rs` — add `reverify` tool handler and definition
  - `crates/ctxgraph-cli/src/commands/reverify.rs` — new command module
  - `crates/ctxgraph-cli/src/main.rs` — add `Reverify` subcommand
- **New types/functions:**
  - `StaleMemory { id: String, memory_type: MemoryType, content: String, decay_score: f64, age_days: u64, suggested_action: StaleAction }`
  - `StaleAction` enum: `Renew`, `Update`, `Expire`, `Keep`
  - `Storage::get_stale_memories(&self, threshold: f64, limit: usize, offset: usize) -> Result<Vec<StaleMemory>>`
- **Indexes:** Add index on `(memory_type, created_at)` for stale memory queries
- **Config changes:** `stale_threshold: f64` (default 0.3) in `[policies.<agent>]`

### Test Plan
- Integration: insert fact 80 days ago (ttl=90d), verify it appears in stale list with decay < 0.3
- Integration: insert pattern 365 days ago, verify it does NOT appear in stale list
- Integration: `ctxgraph reverify list` shows at least one stale memory
- Integration: `ctxgraph reverify renew <id>` updates created_at
- Unit: `get_stale_memories` with threshold=0.0 returns no results (nothing below 0)
- Integration: MCP `reverify` tool returns JSON array of stale memories
- Integration: pagination — offset=10 skips first 10 stale memories

---

## C4: Re-verify CLI command and MCP tool

**Phase:** C
**Priority:** P2
**Effort:** S
**Depends on:** C1, C2, C3

### Description
Finalize the re-verification CLI and MCP interface. This story wires up all the C1-C3 functionality into a cohesive command structure. CLI gets `ctxgraph reverify` with subcommands `list`, `renew <id>`, `update <id>`, `expire <id>`. MCP gets a unified `reverify` tool that can list stale or take action on a specific memory. Also adds a `forget` MCP tool to manually expire a memory immediately. **Review fix:** the `update` command has a defined input format: `{id: string, content?: string, memory_type?: MemoryType}` where at least one of content or memory_type must be provided.

### Acceptance Criteria
1. CLI: `ctxgraph reverify list --threshold 0.3 --limit 20` lists stale memories with decay_score
2. CLI: `ctxgraph reverify renew <id>` renews a specific memory (resets created_at)
3. CLI: `ctxgraph reverify update <id> --content "new value"` updates memory content in-place
4. CLI: `ctxgraph reverify expire <id>` immediately invalidates a memory (sets valid_until to now)
5. MCP: `reverify` tool with `action: "list" | "renew" | "update" | "expire"` and `id` for targeted actions
6. MCP: `reverify` with `action: "update"` accepts `{id: string, content?: string, memory_type?: string}` — at least one field required
7. MCP: `forget` tool expires a memory by ID with `{"id": "..."}` input
8. `ctxgraph stats` output includes re-verification stats: total stale, total renewed, total expired
9. CLI: `ctxgraph reverify --format json` outputs machine-readable JSON

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-cli/src/commands/reverify.rs` — add renew, update, expire subcommands
  - `crates/ctxgraph-cli/src/commands/mod.rs` — register reverify module
  - `crates/ctxgraph-cli/src/main.rs` — add `Reverify` enum with action subcommands
  - `crates/ctxgraph-mcp/src/tools.rs` — finalize `reverify` and add `forget` tool definitions
- **New types/functions:**
  - `Storage::update_memory(&self, id: &str, content: Option<&str>, memory_type: Option<MemoryType>) -> Result<()>`
  - `Storage::expire_memory(&self, id: &str) -> Result<()>`
  - `commands::reverify::run(action)`, MCP `ToolContext::forget`
- **Config changes:** none

### Test Plan
- Integration: `ctxgraph reverify list` on DB with stale memories returns non-empty
- Integration: `ctxgraph reverify renew <id>` then `reverify list` — renewed memory no longer stale
- Integration: `ctxgraph reverify update <id> --content "new value"` — memory content updated, decay_score reset
- Integration: `ctxgraph reverify update <id>` with no content/memory_type flag returns error
- Integration: `ctxgraph reverify expire <id>` — memory no longer returned by any search
- Integration: MCP `forget` tool with valid ID returns success
- Integration: MCP `forget` with invalid ID returns error
- Integration: `ctxgraph stats` shows re-verification metrics

---

## Phase D: Learn (The Differentiator)

Stories D1a-D4 implement the skills layer that makes agents genuinely better over time.

---

## D1a: Co-occurrence counting for pattern extraction

**Phase:** D
**Priority:** P1
**Effort:** L
**Depends on:** B1, B2, A1

### Description
Implement the candidate generation step for pattern extraction using co-occurrence counting (MVP algorithm — not graph mining). Given a set of compression groups, count how often entity types, entity pairs, and relation triplets (entity_a, relation, entity_b) appear across groups. Co-occurrence counts above a configurable `min_occurrence_count` (default 3) become pattern candidates. This is the first of two stories that replace the original D1 — the second (D1b) handles description generation. The algorithm is: (1) iterate compression groups, (2) extract entity types and relation triplets from each group's edges, (3) count co-occurrences in a HashMap, (4) filter by min_occurrence_count, (5) return ranked candidates. No LLM required.

### Acceptance Criteria
1. `PatternExtractor` struct analyzes compression groups and returns co-occurrence counts
2. Counts three types of co-occurrences: entity type frequency, entity pair frequency, relation triplet frequency
3. Candidates with count >= `min_occurrence_count` (default 3, configurable) are returned as `PatternCandidate`
4. `PatternCandidate { id: String, entity_types: Vec<String>, entity_pair: Option<(String, String)>, relation_triplet: Option<(String, String, String)>, occurrence_count: u32, source_groups: Vec<String>, confidence: f64, description: Option<String> }` — `description` is `None` after D1a; D1b populates it via LLM generation
5. `PatternExtractorConfig` struct with `min_occurrence_count: u32` (default 3), `min_entity_types: usize` (default 2), `max_patterns_per_extraction: usize` (default 20)
6. Results ranked by occurrence_count descending, capped at `max_patterns_per_extraction`
7. `Storage::get_pattern_candidates(config: &PatternExtractorConfig) -> Result<Vec<PatternCandidate>>` orchestrates extraction

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `PatternCandidate`, `PatternExtractorConfig`
  - `crates/ctxgraph-core/src/pattern.rs` — new module with `PatternExtractor` (pure logic, no I/O)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `get_pattern_candidates`, helper to load compression group edges
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::extract_pattern_candidates` orchestration
- **New types/functions:**
  - `PatternCandidate { id: String, entity_types: Vec<String>, entity_pair: Option<(String, String)>, relation_triplet: Option<(String, String, String)>, occurrence_count: u32, source_groups: Vec<String>, confidence: f64, description: Option<String> }` — `description` is `None` after D1a; D1b populates it
  - `PatternExtractorConfig { min_occurrence_count: u32, min_entity_types: usize, max_patterns_per_extraction: usize }`
  - `CompressionGroupData { compression_id: String, source_episode_ids: Vec<String>, edges: Vec<Edge>, entities: Vec<Entity> }`
  - `PatternExtractor::extract(&self, groups: &[CompressionGroupData], config: &PatternExtractorConfig) -> Vec<PatternCandidate>`
  - `Storage::get_pattern_candidates(&self, config: &PatternExtractorConfig) -> Result<Vec<PatternCandidate>>`
- **Config changes:** `min_occurrence_count`, `max_patterns_per_extraction` in `[policies.<agent>]`

### Test Plan
- Integration: compress 5 episode groups about Docker bugs, extract candidates, verify Docker-related entities appear
- Unit: `PatternExtractor` with 2 compression groups finds no candidates (threshold = 3)
- Unit: `PatternExtractor` with 4 groups sharing same entity pair finds candidate with occurrence_count=4
- Unit: `min_occurrence_count = 1` returns more candidates than default threshold of 3
- Unit: results capped at `max_patterns_per_extraction` even if more candidates exist
- Integration: candidate entity_types and source_groups correctly populated

---

## D1b: Pattern description generation

**Phase:** D
**Priority:** P1
**Effort:** M
**Depends on:** D1a

### Description
Implement description generation for pattern candidates. Given a `PatternCandidate` from D1a, generate a human-readable description that captures the **behavioral insight**, not just the co-occurrence metadata.

CLAUDE.md's vision for the Learn phase is: *"pattern recognized, takes 1 attempt"* and *"agent already knows the user's Docker setup, common pitfalls, preferred fix approach."* Template strings like *"Entity type Component appears in 5 similar contexts"* are metadata, not behavioral knowledge. They're useless for D2 (skill creation) and D4 (learn command) — the entire downstream Learn pipeline depends on descriptions worth reading.

Descriptions are generated via LLM. The input is the pattern candidate's co-occurrence data plus source episode summaries (from compression groups). The output is a 1-2 sentence behavioral description. Volume is bounded: `max_patterns_per_extraction = 20` per cycle, and extraction doesn't run every query (D4's `learn` command is explicit, not automatic). Cost is negligible.

**Architecture:** Same as B1 — LLM call in `Graph` layer, `Storage` only persists. `PatternCandidate` gains a `description: String` field (D1a defines the struct with a placeholder; D1b adds this field and populates it). No parallel type hierarchy.

**Known simplification:** Patterns are stored as entities with `entity_type = "LearnedPattern"` in the `entities` table. This conflates two concepts (entities = things mentioned; patterns = observations about entities) but is acceptable for POC. A dedicated `patterns` table can be added later if needed.

**Retrieval integration:** Patterns stored by D1b are automatically included in the memory budget during retrieval (Phase A4/Budget). Since patterns are small and permanent, they bypass freshness scoring and are always injected. This is not implemented in D1b — it is wired in during the Budget story (A4b/A4c).

### Acceptance Criteria
1. `Graph::generate_pattern_description(candidate: &PatternCandidate, source_summaries: &[String]) -> Result<String>` calls LLM to produce a 1-2 sentence behavioral description
2. Description prompt includes co-occurrence data and source summaries as context. The prompt MUST instruct the LLM to produce descriptions that follow the CLAUDE.md behavioral quality bar — NOT co-occurrence counts. Examples:
   - **GOOD:** *"When debugging Docker networking issues, the agent typically needs to restart the service container and clear the network bridge — avoid assuming the daemon is healthy."*
   - **GOOD:** *"The user prefers using dark mode and resists configuration changes unless the rationale is clearly explained."*
   - **BAD (rejected):** *"Entity type Component appears in 5 similar contexts."* (metadata, not behavioral)
   - **BAD (rejected):** *"Entity pair (User, Postgres) appears 3 times across source summaries."* (counts, not insight)
3. `Storage::store_pattern(&self, candidate: &PatternCandidate) -> Result<String>` persists pattern as entity with `entity_type = "LearnedPattern"`, `memory_type = Pattern` (never expires), and the LLM-generated description. The entity `name` field is set to the first 80 characters of the description (truncated at word boundary).
4. `Storage::get_patterns() -> Result<Vec<PatternCandidate>>` retrieves all stored patterns with their descriptions populated
5. Patterns have `memory_type: Pattern` and `ttl = None` (never expires) — correct per CLAUDE.md
6. LLM call failure returns error, pattern is not stored, extraction can be retried
7. Empty candidate list returns empty result (not error)

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `description: Option<String>` field to `PatternCandidate` (D1a already defined this field as `None`; D1b populates it)
  - `crates/ctxgraph-core/src/pattern.rs` — add `generate_description` (pure LLM call, no I/O)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `store_pattern`, `get_patterns`
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::extract_and_describe_patterns` orchestration (D1a + D1b combined pipeline)
- **New types/functions:**
  - `Graph::generate_pattern_description(candidate: &PatternCandidate, source_summaries: &[String]) -> Result<String>`
  - `Storage::store_pattern(&self, candidate: &PatternCandidate) -> Result<String>`
  - `Storage::get_patterns(&self) -> Result<Vec<PatternCandidate>>`
- **Config changes:** none (uses Graph's default model, per-agent override deferred to A5)

### Test Plan
- Unit: `generate_pattern_description` produces 1-2 sentence behavioral insight (not co-occurrence counts or entity names)
- Unit: description mentions entities and relations from the candidate, not just numbers
- Unit: description does NOT contain phrases like "appears N times" or "entity type" (rejected patterns)
- Unit: description length is 1-2 sentences (< 200 chars)
- Unit: entity `name` field is truncated description at 80 chars (word boundary)
- Integration: full pipeline — compress groups (B1) → extract candidates (D1a) → generate descriptions (D1b) → store patterns
- Integration: stored pattern has `memory_type = Pattern` and `ttl = None`
- Integration: `get_patterns` returns previously extracted patterns with descriptions
- Unit: LLM call failure (mocked) returns error, no pattern stored
- Integration: `learn` command with no pattern candidates returns empty list gracefully

---

## D2: Skill creation and evolution

**Phase:** D
**Priority:** P1
**Effort:** L
**Depends on:** D1b, A5

### Description
Build on extracted patterns to create Skills — behavioral knowledge about what worked, what failed, and what the user preferred. A Skill is a higher-level abstraction than a pattern: it encodes an actionable rule. Skills are created when patterns show consistent success/failure signals.

**Skill synthesis uses LLM** (not mechanical transform). A `PatternCandidate` has co-occurrence data (`entity_types`, `entity_pair`, `relation_triplet`, `occurrence_count`, `description`). Turning that into a `trigger_condition` ("when to apply") and `action` ("what to do") requires understanding the behavioral insight — the same reasoning we applied in B1 (summaries) and D1b (pattern descriptions). Template approaches produce metadata, not knowledge. CLAUDE.md defines skills as: *"What worked → do this pattern again. What failed → never do this again. What the user preferred → always do it this way."* The skill's `trigger_condition` and `action` fields ARE that knowledge — they can't be mechanically derived.

**Architecture:** Same as B1/D1b — LLM call lives in `Graph` layer (orchestration), not in `Storage` (persistence). `SkillCreator` is pure logic that builds a draft Skill struct from pattern data. `Graph::create_skills_from_patterns` orchestrates: draft skills → LLM populates behavioral fields → store. Storage only handles insert/read.

**Fallback:** If LLM fails, skill creation returns error. No partial/draft skills stored. Retry on next `learn` cycle.

**Configurable relations:** success/failure relation names are configurable via `MemoryPolicyConfig` (not hardcoded). Default success relations: `["fixed", "resolved", "success"]`; default failure relations: `["deprecated", "failed", "abandoned"]`.

**Lifecycle:** Skills never expire (they're proven behaviors per CLAUDE.md). Skills can be superseded: if new evidence contradicts a skill, it gets a `superseded_by` link to the updated skill. Skills with `superseded_by` set are excluded from retrieval but kept for audit. Skills are only created when explicitly triggered (via D4's `learn` command), not automatically.

**Provenance (Layer 2 — perishable):** Skills carry optional provenance tracking WHY the skill exists (reasoning, alternatives rejected, assumptions, context facts). Provenance is auto-generated from the pattern's source data during creation — not a manual LLM call. Provenance has its own TTL (default 180 days) separate from the skill core. See `docs/planning/design-note-skill-provenance-ttl.md` for full design. Provenance is stored as a JSON column on the skills table (no separate table — POC simplification). Provenance renewal uses its own `renewal_count` field (separate from entity/edge `renewal_count` in C2 — skills are not entities).

**Deferred (POC):**
- Provenance-entity linking (`skill_provenance_edges` table for automatic context shift detection) — deferred per design note open question #2
- `success_count`/`failure_count` live updates after creation — skills start with counts from source patterns; post-creation tracking is a future enhancement
- Provenance LLM re-evaluation when expired — deferred (design note open question #4)

### Acceptance Criteria
1. `Skill` struct with fields: `id`, `name`, `description`, `trigger_condition` (when to apply), `action` (what to do), `success_count`, `failure_count`, `confidence`, `superseded_by: Option<String>`, `created_at`, `entity_types: Vec<String>`, `provenance: Option<SkillProvenance>`
2. `Graph::create_skills_from_patterns(patterns: &[PatternCandidate], success_relations: &[String], failure_relations: &[String]) -> Result<Vec<Skill>>` orchestrates: filter patterns by success/failure signals → draft skills → LLM synthesizes `name`, `trigger_condition`, `action`, `description` → store
3. LLM synthesis prompt takes pattern co-occurrence data + source summaries as input. The prompt MUST produce behavioral rules aligned with CLAUDE.md — NOT metadata. Examples:
   - **GOOD `trigger_condition`:** *"When debugging Docker networking issues involving container-to-container connectivity"*
   - **GOOD `action`:** *"Restart the service container, clear the network bridge, verify DNS resolution — do NOT assume the daemon is healthy"*
   - **BAD (rejected):** *"When entity types [Component, Network] appear together"* (metadata, not behavioral)
   - **BAD (rejected):** *"Apply pattern 3"* (meaningless without context)
4. `MemoryPolicyConfig` has `success_relations: Vec<String>` (default `["fixed", "resolved", "success"]`) and `failure_relations: Vec<String>` (default `["deprecated", "failed", "abandoned"]`)
5. Skills stored in a new `skills` table (no `memory_type` column — skills are not entities; they never expire by design). FTS5 index on `name` + `description`
6. `Storage::create_skill(skill: &Skill) -> Result<String>` and `Storage::list_skills() -> Result<Vec<Skill>>` and `Storage::search_skills(query: &str) -> Result<Vec<Skill>>`
7. When new evidence contradicts a skill, `Storage::supersede_skill(old_id, new_id)` updates old skill's `superseded_by` field
8. Skills with `superseded_by` set are excluded from retrieval but kept for audit
9. `Skill.confidence` = `success_count as f64 / (success_count + failure_count) as f64` (at creation time; post-creation confidence updates deferred)
10. Skill provenance: `Option<SkillProvenance>` with fields `reasoning`, `alternatives_rejected`, `assumptions`, `context_facts`, `verified_at`, `expires_at`, `renewal_count`. Auto-generated from pattern source data at creation. Stored as JSON column.
11. Provenance TTL defaults: 180 days for reasoning (configurable via `provenance_ttl_days`), 90 days for context_facts (configurable via `context_ttl_days`) — both in `MemoryPolicyConfig` per agent

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `Skill`, `SkillProvenance` structs; add `success_relations`, `failure_relations`, `provenance_ttl_days`, `context_ttl_days` to `AgentPolicy`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 007 (skills table + FTS5 on name + description)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `create_skill`, `list_skills`, `supersede_skill`, `search_skills`; provenance serialized/deserialized as JSON
  - `crates/ctxgraph-core/src/skill.rs` — new module with `SkillCreator` (pure logic: draft skills from pattern data, filter by success/failure signals)
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::create_skills_from_patterns` (orchestration + LLM synthesis + storage)
- **New types/functions:**
  - `Skill { id: String, name: String, description: String, trigger_condition: String, action: String, success_count: u32, failure_count: u32, confidence: f64, superseded_by: Option<String>, created_at: DateTime<Utc>, entity_types: Vec<String>, provenance: Option<SkillProvenance> }`
  - `SkillProvenance { reasoning: String, alternatives_rejected: Option<String>, assumptions: Option<String>, context_facts: Option<String>, verified_at: DateTime<Utc>, expires_at: DateTime<Utc>, renewal_count: u32 }` (stored as JSON column on skills table)
  - `SkillCreator::draft_skills(patterns: &[PatternCandidate], success_relations: &[String], failure_relations: &[String]) -> Vec<DraftSkill>` — pure logic, no I/O, no LLM
  - `DraftSkill { entity_types: Vec<String>, success_count: u32, failure_count: u32, source_pattern_ids: Vec<String>, source_summaries: Vec<String> }` — intermediate struct before LLM synthesis
  - `Graph::create_skills_from_patterns(patterns: &[PatternCandidate], success_relations: &[String], failure_relations: &[String]) -> Result<Vec<Skill>>` — full pipeline
  - `Graph::synthesize_skill(draft: &DraftSkill) -> Result<(String, String, String, String)>` — LLM call returning `(name, trigger_condition, action, description)`
  - `Storage::create_skill(&self, skill: &Skill) -> Result<String>`
  - `Storage::supersede_skill(&self, old_id: &str, new_id: &str) -> Result<()>`
- **Config changes:** `success_relations`, `failure_relations`, `provenance_ttl_days` (default 180), `context_ttl_days` (default 90) in `[policies.<agent>]`

### Test Plan
- Integration: extract patterns from 5 compression groups about Docker fixes, create skills, verify skill has behavioral `trigger_condition` and `action` (not entity type names)
- Integration: skill `name`, `trigger_condition`, `action`, `description` are all human-readable behavioral text (not metadata like "entity types: [Component, Network]")
- Integration: `list_skills` returns active skills (not superseded ones)
- Integration: supersede a skill, verify old skill has `superseded_by` set, new skill is active
- Unit: `SkillCreator::draft_skills` with no success/failure signals returns empty vec
- Integration: custom success_relations `["resolved", "success"]` correctly filter patterns
- Integration: `search_skills` via FTS5 finds relevant skills by name or description
- Integration: skill created with provenance, provenance JSON round-trips correctly
- Unit: `SkillProvenance.expires_at` = `created_at + provenance_ttl_days`
- Unit: `Skill.confidence` = `success_count / (success_count + failure_count)` at creation
- Unit: LLM call failure (mocked) returns error, no skill stored
- Integration: empty pattern list returns empty skill list (not error)

---

## D3: Cross-session skill persistence and sharing

**Phase:** D
**Priority:** P2
**Effort:** M
**Depends on:** D2

### Description

**Persistence.** Skills persist across sessions automatically via SQLite persistence — no special handling needed beyond D2's storage.

**Scope and ownership.** Add a `scope` field to Skill: `"private"` (agent-only) or `"shared"` (available to all agents using the same graph DB). Add a `created_by_agent` column to the `skills` table (set at creation time from the calling context — D4's `learn` command or CLI provides the agent name). D3's Storage functions accept `created_by_agent` as a parameter.

**Sharing model.** Sharing is one-way (private → shared, not reversible) as a POC simplification — shared skills may have been consumed by other agents' retrievals, so un-sharing could cause inconsistencies. Reversible sharing can be added later.

**Budget integration.** Skills consume budget tokens. Skills are retrieved via FTS5 search on `name` + `description`, scored at a fixed floor of 0.8 (higher than patterns' 0.5 — skills are actionable rules, more valuable than raw patterns), and go through the same `enforce_budget` as entity/edge candidates. This keeps a single retrieval pipeline with no separate injection step.

### Acceptance Criteria
1. `Skill` struct has `scope: SkillScope` field with variants `Private` and `Shared`
2. `Skill` struct has `created_by_agent: String` field (set at creation time from calling context)
3. `Storage::share_skill(id: &str) -> Result<()>` changes scope from Private to Shared
4. `retrieve_for_context` includes shared skills for any agent, private skills only for owning agent
5. `skills` table has `scope` and `created_by_agent` columns (ALTER TABLE, no new table)
6. Skills retrieved via FTS5 on `name` + `description` enter the candidate set with a floor score of 0.8 and go through the same `enforce_budget` as entity/edge candidates

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/types.rs` — add `SkillScope` enum, `scope` and `created_by_agent` fields on `Skill`
  - `crates/ctxgraph-core/src/storage/migrations.rs` — add migration 008 (ALTER TABLE skills ADD COLUMN scope, created_by_agent)
  - `crates/ctxgraph-core/src/storage/sqlite.rs` — add `share_skill`, update `get_skills_for_agent` to query skills table with WHERE filter on `scope` and `created_by_agent` (no join on skill_sources)
  - `crates/ctxgraph-core/src/graph.rs` — update `retrieve_for_context` to query skills table via FTS5 and include matching skills as `ScoredCandidate` entries with 0.8 floor score before `enforce_budget`
- **New types/functions:**
  - `SkillScope` enum: `Private`, `Shared`
  - `Storage::share_skill(&self, id: &str) -> Result<()>`
  - `Storage::get_skills_for_agent(&self, agent_name: &str) -> Result<Vec<Skill>>`
- **Config changes:** none

### Test Plan
- Integration: create private skill for agent "coding", verify agent "coding" sees it, agent "finance" does not
- Integration: `share_skill` makes skill visible to all agents
- Integration: skills matching a query appear in `retrieve_for_context` output with score ≥ 0.8
- Integration: skills participate in budget enforcement (large skill sets don't exceed budget_tokens)
- Unit: skill with no scope defaults to Private

---

## D4: Learn CLI command and MCP tool

**Phase:** D
**Priority:** P2
**Effort:** S
**Depends on:** D1a, D1b, D2, D3

### Description
Expose the learning pipeline via CLI and MCP. Both interfaces trigger the same underlying orchestration (CLI for humans, MCP for agents).

**Explicit trigger:** Per CLAUDE.md, skills are created from compressed experience patterns — not from every episode. The learn command is an explicit invocation (manual or agent-driven) that runs the full pipeline on demand.

**Pipeline:** The learn command orchestrates the following steps:
1. Load compression groups via `Storage::get_pattern_candidates` (D1a prerequisite data)
2. Extract pattern candidates via `PatternExtractor::extract(groups, config)` (D1a)
3. Generate descriptions via `Graph::generate_pattern_description` for each candidate (D1b)
4. Dedup: retrieve existing patterns via `Storage::get_patterns`, skip candidates that already exist (same entity_pair + relation_triplet)
5. Create skills from new + existing patterns via `Graph::create_skills_from_patterns` (D2)
6. Supersession: check new skills against existing skills; if entity_types overlap and actions differ, supersede old skill via `Storage::supersede_skill`
7. Assign scope from `--scope` flag (D3), default Private

This fulfills the design philosophy: "Phase D's learn command is itself a retrieval decision point (retrieve patterns before creating skills)."

**CLI/MCP parity:** Both expose the same pipeline. CLI is for human operators; MCP is for agents. CLI shows a summary of new patterns found and skills created/updated. Add skill display to `ctxgraph stats` output.

**MCP tool note:** `list_skills` and `share_skill` MCP tools extend beyond CLAUDE.md's high-level MCP tool list — they provide skill management capabilities needed for the Learn workflow.

### Acceptance Criteria
1. CLI: `ctxgraph learn` runs full learning pipeline and reports new patterns and skills
2. CLI: `ctxgraph learn --dry-run` shows what would be learned without persisting
3. CLI: `ctxgraph learn --scope shared` creates skills as shared by default
4. CLI: `ctxgraph learn --limit 10` caps the number of skills created per run (pattern extraction count is controlled by D1a's `max_patterns_per_extraction` config)
5. CLI: `ctxgraph learn --format json` outputs machine-readable JSON with `{patterns_found, patterns_new, skills_created, skills_updated, skill_ids}`
6. CLI: `ctxgraph learn --agent coding` specifies which agent owns created skills (defaults to config's `default_agent` or `"assistant"`)
7. CLI: `ctxgraph stats` output includes skill count, pattern count, and shared vs private breakdown
8. MCP: `learn` tool runs full pipeline: extract patterns from recent compressions, create/update skills, return count of created/updated skills
9. MCP: `list_skills` tool returns all active (non-superseded) skills for the agent
10. MCP: `share_skill` tool with `{id: "..."}` changes skill scope to shared
11. Dedup: learn retrieves existing patterns before extraction; candidates matching existing patterns (same entity_pair + relation_triplet) are skipped
12. Supersession: learn checks new skills against existing skills; if entity_types overlap and actions differ, old skill is superseded via `Storage::supersede_skill`

### Technical Requirements
- **Files to create/modify:**
  - `crates/ctxgraph-core/src/graph.rs` — add `Graph::run_learning_pipeline` (orchestrates D1a→D1b→dedup→D2→D3)
  - `crates/ctxgraph-cli/src/commands/learn.rs` — new command module
  - `crates/ctxgraph-cli/src/commands/mod.rs` — register learn module
  - `crates/ctxgraph-cli/src/main.rs` — add `Learn` subcommand
  - `crates/ctxgraph-mcp/src/tools.rs` — add `list_skills`, `share_skill`, `learn` tool definitions
  - `crates/ctxgraph-cli/src/commands/stats.rs` — extend to include skill/pattern counts
- **New types/functions:**
  - `Graph::run_learning_pipeline(agent_name: &str, scope: SkillScope, limit: usize) -> Result<LearningOutcome>` — orchestrates the full D1a→D1b→dedup→D2→D3 pipeline
  - `commands::learn::run(dry_run, scope, limit, agent, format)` — CLI entry point, calls `Graph::run_learning_pipeline`
  - MCP `learn` tool handler — calls `Graph::run_learning_pipeline(agent_name, scope, limit)`, agent_name from session context
  - MCP `list_skills` tool handler — calls `Storage::get_skills_for_agent`
  - MCP `share_skill` tool handler — calls `Storage::share_skill`
- **New types:** `LearningOutcome { patterns_found: usize, patterns_new: usize, skills_created: usize, skills_updated: usize, skill_ids: Vec<String> }`
- **Config changes:** none

### Test Plan
- Integration: `ctxgraph learn` on DB with compressed episodes creates patterns and skills
- Integration: `ctxgraph learn --dry-run` outputs "would create N patterns, M skills" without writing
- Integration: `ctxgraph learn --scope shared` creates skills with Shared scope
- Integration: `ctxgraph learn --limit 5` creates at most 5 skills (not patterns)
- Integration: `ctxgraph learn --format json` outputs JSON with `patterns_found`, `patterns_new`, `skills_created`, `skills_updated`, `skill_ids`
- Integration: `ctxgraph learn --agent coding` sets `created_by_agent = "coding"` on new skills
- Integration: MCP `list_skills` returns JSON array of active skills
- Integration: MCP `share_skill` with valid ID returns success
- Integration: `ctxgraph stats` shows "Skills: 5 (3 shared, 2 private)"
- Integration: MCP learn tool returns created/updated skills count
- Edge case: `ctxgraph learn` on DB with no compressed episodes returns empty results (not error)
- Edge case: `ctxgraph learn` when all patterns already exist creates no new patterns but may still create/update skills
- Edge case: MCP `share_skill` with invalid ID returns error
- Edge case: MCP `share_skill` on already-shared skill is idempotent (returns success, no-op)

---

# Dependency Graph

```
A1 (TTL + memory_type fields)
├── A2 (decay_score)
│   └── A4b (scoring + ranking) ── depends on A1, A2, A3, A4a
├── A3 (usage_count + last_recalled_at tracking)
│   ├── A4b (scoring + ranking)
│   ├── A6 (TTL enforcement/cleanup) ── depends on A1, A2, A3
│   └── C2 (implicit renewal) ── depends on A1, A3
├── A4a (FTS5 + graph candidate retrieval) ── depends on A1, A3
│   └── A4b (scoring + ranking)
├── A4c (budget enforcement) ── depends on A4a, A4b
└── A5 (per-agent policies) ── depends on A1, A4c
    └── (A4c uses AgentPolicy from A5 at runtime; A5 loads policies at init)

B1 (compression pipeline) ── depends on A1, A3
├── B2 (relationship inheritance) ── depends on B1
├── B3 (compression triggers, lazy interval) ── depends on B1, A5, A6
└── B4 (compress CLI/MCP) ── depends on B1, B3

C1 (contradiction detection) ── depends on A1, A3
C2 (implicit renewal, uses renewal_count) ── depends on A1, A3
C3 (active re-verification, configurable threshold) ── depends on A1, A2, A3
C4 (reverify CLI/MCP) ── depends on C1, C2, C3

D1a (co-occurrence counting) ── depends on B1, B2, A1
│   └── D1b (description generation) ── depends on D1a
│       └── D2 (skill creation, LLM synthesis) ── depends on D1b, A5
│           └── D3 (cross-session sharing, budget integration) ── depends on D2
│               └── D4 (learn CLI/MCP, dedup, supersession) ── depends on D1a, D1b, D2, D3
```

# Effort Summary

| Phase | Stories | P0 | P1 | P2 | Total Effort |
|-------|---------|----|----|-----|-------------|
| A     | 8       | 6  | 2  | 0   | 1L + 5M + 2S |
| B     | 4       | 1  | 3  | 0   | 1L + 2M + 1S |
| C     | 4       | 1  | 1  | 2   | 1L + 2M + 1S |
| D     | 5       | 0  | 3  | 2   | 2L + 2M + 1S |
| Total | 21      | 8  | 9  | 4   | 5L + 11M + 5S |

# Migration Plan

| Migration | Story | Changes |
|-----------|-------|---------|
| 003 | A1 | Add `memory_type`, `ttl_seconds` to entities + edges + episodes (idempotent: UPDATE WHERE ttl_seconds IS NULL) |
| 004 | A3 | Add `usage_count`, `last_recalled_at` to entities + edges |
| 005 | A6 | Add `archived_entities`, `archived_edges`, `system_metadata` tables (for TTL cleanup); add index on `(memory_type, created_at)` |
| 006 | B1 | Add `compression_id` to episodes |
| 007 | D2 | Add `skills` table + FTS5 index on description |
| 008 | D3 | Add `scope` and `created_by_agent` columns to skills table |
| 009 | C2 | Add `renewal_count INTEGER NOT NULL DEFAULT 0` to entities + edges |

> **Note:** Migration numbers reflect implementation phase order. Implement in numerical sequence.

# Changes from Round 1

## Critical Fixes Applied

1. **A4 split into A4a/A4b/A4c** — retrieval, scoring, and budget enforcement are now separate stories with clear boundaries. A4b scoring uses `usage_count` (recall frequency) not `renewal_count`.

2. **A3 trimmed** — `renewal_count` deferred to C2 where it's consumed (C2 adds its own migration). A3 only adds `usage_count` + `last_recalled_at`. Removed `touch_many`, `get_usage_stats`, `MemoryTable` enum, and premature indexes. Effort M→S.

3. **C2 uses `renewal_count` not `usage_count`** — renewal limit checks `renewal_count >= max_renewals`, not `usage_count`. Fixes the logical conflict where frequently-used memories couldn't be renewed.

4. **B3 compression no longer runs every query** — lazy interval-based: checks every N queries (default 50), uses `last_compression_at` timestamp and `compression_in_progress` flag to prevent redundant/concurrent runs.

5. **A6 TTL enforcement added** — new P0 story for deleting/archiving expired memories after grace_period. Without this, data grows indefinitely.

6. **D1 split into D1a/D1b** — D1a uses co-occurrence counting (concrete algorithm, configurable thresholds). D1b uses LLM-based description generation (not template).

## Non-Critical Fixes Applied

- **A1**: Migration 003 uses `WHERE ttl_seconds IS NULL` for idempotency
- **A2**: Explicit formula documentation, edge case for ttl=0
- **A3**: Trimmed to `usage_count` + `last_recalled_at` only. `renewal_count` deferred to C2. Removed `touch_many`, `get_usage_stats`, `MemoryTable` enum, premature indexes. Effort M→S.
- **B1**: Rewritten — LLM-based summary generation (not template). `memory_type: Fact` for compressed summaries (not Pattern). LLM call in Graph layer, Storage only persists. LLM failure leaves source episodes untouched (retry next cycle). Removed `compression_groups` table, `compressed_at` field.
- **B2**: Inherited edges get new IDs, metadata merge strategy defined, uniqueness constraint added
- **C1**: Uses entity_id for matching (entity_name as fallback), adds confidence threshold, entity name normalization
- **C3**: `stale_threshold` configurable per agent (default 0.3), pagination added, `StaleAction::Keep` variant added
- **C4**: Update format defined: `{id, content?, memory_type?}`, CLI `--content` flag specified
- **D2**: Skill synthesis uses LLM (not mechanical transform). `SkillCreator::draft_skills` produces `DraftSkill` intermediates; `Graph::synthesize_skill` calls LLM for behavioral `trigger_condition`/`action`. Skills table has no `memory_type` column (skills are not entities). `confidence` formula defined. Provenance `renewal_count` separate from C2's entity/edge counter. LLM failure returns error. Deferred: provenance-entity linking, `success_count`/`failure_count` live updates, provenance LLM re-evaluation.
- **D3**: Simplified — `skill_sources` table removed, `scope`/`created_by_agent` columns added to skills table via ALTER TABLE (migration 008). Cross-session persistence AC removed (automatic via SQLite). Skill budget integration defined: skills retrieved via FTS5 with 0.8 floor score, enter same `enforce_budget` pipeline as entities/edges. `graph.rs` added to files to modify. Sharing documented as one-way (irreversible) POC simplification. `created_by_agent` passed from calling context (D4).
- **D4**: Pipeline orchestration defined (D1a→D1b→dedup→D2→D3). `--limit` caps skills created (not patterns). `--format json` and `--agent` flags added. Dedup against existing patterns and supersession check against existing skills. Edge case tests added.
- **A5**: `set_policy` semantics clarified (session override, not persisted)
