# MiniMax M2.7 Review — Round 1

# Senior Technical Review: ctxgraph4agent User Stories

## Individual Story Assessments

---

### A1: Add ttl and memory_type fields to Entity and Edge
**Verdict: PASS**

**Issues:**
- Migration 003 `UPDATE` clause needs idempotency — running migration twice on existing rows could overwrite user-customized TTLs with defaults
- No mention of handling the case where `entity_type` mapping returns a type not in the `MemoryType` enum

**Suggestions:**
- Migration should use `UPDATE ... WHERE ttl_seconds IS NULL` to preserve explicit values
- Add a fallback: if entity_type doesn't map, default to `Fact`
- Consider adding a `memory_type_source` field to track whether TTL was explicitly set or defaulted (useful for UI to show "defaulted" vs "user-configured")

---

### A2: Implement decay_score computation
**Verdict: PASS**

**Technical Notes:**
- The exponential decay formula needs explicit definition: `exp(-λ * age)` where `λ = ln(2) / half_life`
- Edge case: if `ttl = 0`, what happens? Should probably return 0.0 immediately
- `decay_linear` needs careful implementation: `max(0.0, 1.0 - (age / ttl))`

**Suggestions:**
- Add explicit unit tests for boundary conditions: `age = 0`, `age = ttl`, `age > ttl`, `ttl = None`
- Document the formula in a doc comment on the function

---

### A3: Add usage_count and last_recalled_at tracking
**Verdict: PASS**

**Issues:**
- `touch_entity` and `touch_edge` need to be in the same transaction as the retrieval that triggered them to avoid race conditions under concurrent access
- `get_usage_stats()` needs to clarify: does it count expired/deleted records or only active ones?

**Suggestions:**
- Use SQLite's `RETURNING` clause (available since 3.35.0) to verify the update affected exactly one row
- Add index on `(memory_type, usage_count)` for the stats query
- Consider `touch_many` batch operation for efficiency when touching multiple memories in `retrieve_for_context`

---

### A4: Budget-aware retrieval ranking
**Verdict: NEEDS_FIX**

**Critical Issues:**
1. **Score formula conflates renewal with usage**: `usage_count` is used in both A2's renewal logic (counting renewals) and A4's scoring (recency bonus). If a memory was renewed 5 times but never used, it incorrectly gets a usage bonus.

2. **Token estimation is naive**: `text.len() / 4` is a rough approximation. Actual token counts vary wildly by content type. Should use a proper tokenizer or at minimum acknowledge this is a ceiling estimate.

3. **FTS5 + graph traversal not specified**: How are candidates retrieved before ranking? What queries are used? How is the initial candidate set limited?

4. **Pattern inclusion logic is ambiguous**: "always included unless budget is exceeded" — which patterns? All patterns in the DB? Patterns matching the query? This could return thousands of patterns.

**Suggestions:**
- Separate `renewal_count` from `usage_count` in A3 to avoid this conflict
- Document the FTS5 query strategy
- Limit patterns to those relevant to the query, or add a configurable `max_patterns_included` cap
- Add `token_budget_spent` to the return type so callers know how much budget was consumed

---

### A5: Per-agent memory policies via ctxgraph.toml
**Verdict: PASS**

**Issues:**
- `set_policy` MCP tool: if policy is changed at runtime, does it persist? If yes, where? If no, is this just a session override?
- No conflict resolution when two agents have overlapping policies

**Suggestions:**
- Clarify `set_policy` persistence semantics
- Add policy versioning or timestamps to track when policies changed
- Consider a `policy_history` table for auditability
- Add validation: if `compress_after < 7` days, warn that this may be too aggressive

---

### B1: Episode compression pipeline
**Verdict: PASS**

**Issues:**
- "Generated summary via extraction of shared entities/relations rather than LLM call" is underspecified. What algorithm generates the summary text?

**Suggestions:**
- Define the summary generation algorithm explicitly (e.g., extract all unique entities, list the most common relations, format as "In [timeframe], [entity] did [top N relations]")
- Consider storing the compression metadata (source episode IDs, time range) as structured JSON, not just as linked records
- Add `compressed_at: DateTime<Utc>` to the summary episode for audit

---

### B2: Relationship inheritance from compressed nodes
**Verdict: PASS**

**Issues:**
- Edge deduplication: if two source edges have the same (source, target, relation) but different confidence scores, `max()` is reasonable, but what about different metadata? Should metadata be merged or concatenated?

**Suggestions:**
- Define metadata merge strategy: concatenate arrays, union JSON objects, prefer newer timestamps
- Add uniqueness constraint on `(source_id, target_id, relation, compression_id)` in the `compression_groups` table to prevent duplicate insertions
- Consider: should inherited edges get a new ID, or preserve the original edge IDs with a reference? New ID is cleaner for deletion/TTL purposes.

---

### B3: Compression triggers
**Verdict: NEEDS_FIX**

**Critical Issues:**
1. **Performance hazard**: Running compression checks at query time (`retrieve_for_context`) means every retrieval could trigger compression, adding unpredictable latency. If compression compresses 50 episodes, this could take seconds.

2. **Trigger algorithm underspecified**: "Group them by source and compress" — what if episodes have different sources? What's the grouping heuristic?

3. **No size-based trigger implementation details**: When `max_episodes` is exceeded, how is the batch selected? Oldest first? Random sample? By source?

**Suggestions:**
- Move compression triggers to a background task or run them every N queries, not every query
- Define a `CompressionStrategy` enum with variants: `BySource`, `ByEntityCluster`, `OldestFirst`
- Add a `last_compression_at` timestamp to avoid re-checking if compression ran recently
- Add `compression_in_progress` flag to prevent concurrent compression runs

---

### B4: Compression CLI command and MCP tool
**Verdict: PASS**

**Issues:**
- None

**Suggestions:**
- Add `--quiet` flag to suppress output for scripting
- Consider adding `--format json` for machine-readable output in both CLI and MCP

---

### C1: Passive re-verification (detect contradictions at write time)
**Verdict: PASS**

**Issues:**
- Contradiction detection by entity name + relation may have false positives: "Alice" in episode 1 may not be the same "Alice" in episode 5 (different context)

**Suggestions:**
- Consider using entity ID (if stable across episodes) instead of entity name for matching
- Add a confidence threshold: don't flag contradictions if the existing edge has very low confidence (e.g., < 0.2)
- Consider fuzzy matching for entity names (normalization to lowercase, trim whitespace)
- Document the contradiction policy: does newer always win, or does higher confidence win regardless of time?

---

### C2: Implicit TTL renewal (recalled and used -> auto-renew)
**Verdict: NEEDS_FIX**

**Critical Issue:**
- **A3/A4 conflict (same as noted in A4)**: `usage_count` is used for:
  - A4's scoring bonus: `1.0 + 0.1 * ln(1 + usage_count)`
  - C2's renewal limit: `if usage_count > max_renewals, renewal is denied`
  
  This means a frequently-used fact (used 100 times) would hit the renewal limit immediately, even though the usage bonus suggests it's valuable and should be renewed.

**Suggestions:**
- Add a separate `renewal_count` field (distinct from `usage_count`)
- Alternatively, define `renewal_limit` as a separate counter that only increments when renewal actually occurs
- Clarify: does recalling via `retrieve_for_context` always count as "used" for renewal purposes, or only when the memory actually appears in the returned results?

---

### C3: Active re-verification (surface stale memories for confirmation)
**Verdict: PASS**

**Issues:**
- `decay_score < 0.3` threshold is hardcoded in the story but should be configurable per-agent

**Suggestions:**
- Add `stale_threshold` to `MemoryPolicyConfig` (default 0.3)
- Consider adding `StaleAction::Keep` variant for cases where the system shouldn't suggest any action
- Add pagination to `get_stale_memories` for agents with thousands of stale memories

---

### C4: Re-verify CLI command and MCP tool
**Verdict: PASS**

**Issues:**
- `update <id>` is mentioned but the input format for updates is not defined

**Suggestions:**
- Define the update format: `{id, content?, memory_type?}` where content is the new text
- Consider adding `ctxgraph reverify update <id> --content "new value"` CLI support
- Add `--format` flag for JSON output

---

### D1: Pattern extraction from compressed experiences
**Verdict: NEEDS_FIX**

**Critical Issues:**
1. **Algorithm is undefined**: "purely structural analysis" is not a specification. What graph algorithms are used? Frequent subgraph mining? Co-occurrence counting? Sequence mining?

2. **Thresholds are arbitrary**: "same entity type appearing in >3 compression groups" — why 3? This threshold should be configurable.

3. **Output quality undefined**: How is `ExtractedPattern.description` generated? If it's not an LLM call, what algorithm produces human-readable text?

**Suggestions:**
- Specify the algorithm: candidate generation → filtering → pattern ranking → description generation
- Consider using established algorithms like gSpan for frequent subgraph mining, or simpler co-occurrence for MVP
- Define a `PatternExtractorConfig` with thresholds: `min_occurrence_count`, `min_entity_types`, `max_patterns_per_extraction`
- For MVP, consider generating descriptions as template strings: "Entity [type] appears in [count] similar contexts"

---

### D2: Skill creation and evolution
**Verdict: PASS**

**Issues:**
- "Edges with relation 'fixed' = success, 'deprecated' = failure" — this assumes these relation names exist in the schema. Are they guaranteed to exist? What if the user's extraction schema uses different terms?

**Suggestions:**
- Make success/failure relation names configurable in `MemoryPolicyConfig`
- Add `SkillCreatorConfig` with `success_relations: Vec<String>`, `failure_relations: Vec<String>`
- Consider: should skills be created automatically, or only when explicitly triggered (via D4's `learn` command)?

---

### D3: Cross-session skill persistence and sharing
**Verdict: PASS**

**Issues:**
- "Private skills only for owning agent" — how is the owning agent determined? By the `agent_name` field on the skill? By which agent created it?

**Suggestions:**
- Add `created_by_agent: String` field to `Skill` (or derive from `skill_sources`)
- Clarify: can private skills be shared later? (Yes, `share_skill` exists, but should this be reversible?)
- Consider: should `retrieve_for_context` include private skills from other agents if the current agent has `scope: Shared` access to the graph?

---

### D4: Learn CLI command and MCP tool
**Verdict: PASS**

**Issues:**
- None

**Suggestions:**
- Add `--limit N` to control how many patterns/skills are created per run
- Consider `--since <datetime>` to only process compressions since a certain date
- Add `--watch` mode that runs learning pipeline periodically

---

## Overall Assessment

### Overall Verdict: NEEDS_WORK (5 stories require fixes)

### Critical Issues (must address before implementation)

| # | Issue | Impact | Stories |
|---|-------|--------|---------|
| 1 | **Renewal/usage count conflict**: `usage_count` is used for both scoring bonus (A4) and renewal limiting (C2), creating a logical conflict where frequently-used memories can't be renewed. | **C2, A4** | Logic bug |
| 2 | **A4 budget retrieval underspecified**: FTS5 + graph traversal candidate retrieval is not defined; pattern inclusion logic is ambiguous. | **A4** | Incomplete spec |
| 3 | **B3 compression-at-query-time**: Running compression checks and potentially triggering compression during every retrieval is a performance hazard. | **B3** | Performance |
| 4 | **D1 pattern extraction algorithm undefined**: "Purely structural analysis" is not a specification; thresholds are arbitrary; description generation is unspecified. | **D1** | Incomplete spec |
| 5 | **No TTL enforcement story**: Stories define `decay_score` returning 0.0 for expired items, but there's no story for actually deleting or archiving expired data. Expired data accumulates forever. | **Missing** | Data growth |

### Stories to Split

| Story | Reason | Suggested Split |
|-------|--------|-----------------|
| **A4** | Too many concerns: retrieval strategy, scoring, budget enforcement, pattern handling, token estimation | Split into A4a (FTS5 + graph retrieval), A4b (scoring + ranking), A4c (budget enforcement + token counting) |
| **D1** | Pattern extraction is underspecified; description generation adds another undefined component | Split into D1a (frequent co-occurrence counting — concrete), D1b (pattern-to-description generation — needs more thought) |

### Missing Stories

| Missing Story | Description | Priority |
|---------------|-------------|----------|
| **TTL Enforcement/Cleanup** | Scheduled or on-demand job to delete/ archive nodes where `valid_until < now` and `decay_score = 0` for > decay_period. Without this, the SQLite file grows indefinitely. | P0 |
| **Episode Cleanup** | Delete or archive episodes older than `max_episode_age` (configurable, default 90d) regardless of compression status. | P1 |
| **Concurrency Handling** | SQLite behavior under concurrent reads/writes; WAL mode configuration; busy_timeout settings; what happens when compression runs during retrieval? | P1 |
| **Migration Testing** | No story for testing migrations on production data volumes, rolling back failed migrations, or handling migration from v0 to v1 of the schema. | P1 |
| **Error Recovery** | What happens when compression partially fails? When contradiction detection fails? Transaction rollback strategies? | P1 |
| **Budget Metrics** | Story for `ctxgraph stats` to show: total memories by type, avg decay_score by type, expired count, compression ratio (original vs compressed tokens). | P2 |
| **Export/Import** | Ability to export skills, patterns, and policies to share across instances. | P2 |

### SQLite Realism Concerns

1. **Index coverage**: Migrations add columns but don't document what indexes are needed. Expected queries:
   - `WHERE memory_type = ? AND created_at < ?` (for stale retrieval)
   - `WHERE usage_count > ?` (for renewal checks)
   - `WHERE compression_id IS NULL` (for uncompressed episodes)
   - These need indexes.

2. **WAL mode**: SQLite default journal mode is DELETE; for a concurrent agent system, WAL mode should be enabled.

3. **FTS5 maintenance**: FTS5 tables need periodic `OPTIMIZE` and can bloat. No story for FTS maintenance.

4. **Backup strategy**: No story for SQLite `.backup` or online backup API usage.

### Dependency Graph Issues

The dependency graph is correct but underrepresents:
- **A4 depends on A3's data model** (usage_count fields) but the scoring formula needs correction (issue #1)
- **B3 depends on B1's data model** but compression-at-query-time is a performance hazard
- **C2 depends on A3** but the renewal limit mechanism conflicts with A4's scoring

### Positive Aspects

- Well-structured phase breakdown (Foundation → Size Control → Quality → Learning)
- Clear differentiation from Hindsight/SOTA
- Good test plan coverage per story
- Migration plan is complete (003-007)
- Effort estimates are reasonable

### Recommendations

1. **Before Phase A implementation**: Fix the `usage_count` / `renewal_count` conflict (A3 story should add two separate fields, or C2 should use a different counter)

2. **Before Phase B implementation**: Define the compression-at-query-time strategy; consider moving to a periodic background task instead

3. **Before Phase D implementation**: Define the pattern extraction algorithm formally; don't start D1 until the graph analysis algorithms are specified

4. **Add missing P0 story**: TTL Enforcement/Cleanup — this is fundamental to the "stays within budget" thesis