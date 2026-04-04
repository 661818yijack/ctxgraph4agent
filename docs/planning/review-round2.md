# MiniMax M2.7 Review — Round 2

# Round 2 Review: ctxgraph4agent User Stories

## Round 1 Critical Issues Verification

| # | Issue | Status | Verification |
|---|-------|--------|--------------|
| 1 | A4 underspecified, needed splitting | **FIXED** | Split into A4a (FTS5+graph retrieval), A4b (scoring with decay), A4c (budget enforcement). Clear boundaries and distinct responsibilities. |
| 2 | C2 usage_count/renewal_count conflict | **FIXED** | A3 introduces both fields with explicit separation: `usage_count` for recall frequency (A4b scoring), `renewal_count` for TTL renewals (C2 limits). C2 explicitly uses `renewal_count >= max_renewals`. |
| 3 | B3 compression-at-query-time performance hazard | **FIXED** | B3 now uses lazy interval-based triggers: `compression_check_interval` (default 50 queries), `last_compression_at` timestamp, and `compression_in_progress` flag. NOT every query. |
| 4 | D1 pattern extraction algorithm undefined | **FIXED** | Split into D1a (co-occurrence counting algorithm) and D1b (template-based description generation). Both have concrete, implementable algorithms with configurable thresholds. |
| 5 | Missing TTL Enforcement/Cleanup story | **FIXED** | A6 added as P0 priority with grace period, type-based strategies (delete/archive), lazy execution, and concurrent-run prevention. |

**All 5 critical issues are properly addressed.**

---

## New Stories Review

### A4b: Scoring and ranking with decay
**Verdict: PASS**

**Strengths:**
- Composite formula `decay_score * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count))` is mathematically sound
- Pattern floor of 0.5 ensures visibility — good design choice
- Explicitly uses `usage_count` (not `renewal_count`) — consistent with A3 separation
- ln() usage bonus provides diminishing returns: usage_count=0 → 1.0x, usage_count=100 → ~1.46x

**Minor note:** Composite score can theoretically exceed 1.0 (~1.46 max with high usage). Acceptable given Pattern floor keeps them visible, but could document this upper bound.

### A4c: Budget enforcement and token counting
**Verdict: PASS**

**Strengths:**
- Greedy selection with skip-when-exceeded is correct
- `token_budget_spent` field enables caller transparency
- `retrieve_for_context` orchestration is clean
- Single memory exceeding budget is skipped (not rejected) — graceful handling

**Minor note:** Token estimation `text.len() / 4` acknowledged as ceiling estimate, appropriately documented.

### D1b: Pattern description generation
**Verdict: PASS**

**Strengths:**
- Template-based (no LLM dependency) — appropriate for MVP
- Three clear template types: entity type, entity pair, relation triplet
- `memory_type: Pattern` with no TTL — consistent with patterns being eternal
- Descriptions are human-readable and actionable

### A6: TTL enforcement and cleanup
**Verdict: PASS**

**Strengths:**
- Grace period prevents premature deletion
- Type-based strategies: Facts/Experiences → DELETE, Preferences/Decisions → archive
- Patterns explicitly excluded (they don't decay anyway)
- Lazy execution with `last_cleanup_at` and `cleanup_in_progress` flag
- `CleanupResult` provides useful reporting metrics

---

## Dependency Graph Verification

The dependency graph is **correct**. All edges are logically sound:
- A4b correctly depends on A4a (needs candidates before scoring)
- B3 correctly depends on A5 (needs per-agent policy for compression settings)
- D1a correctly depends on B1, B2, A1 (needs compression groups before co-occurrence counting)
- C2 correctly depends on A1, A3 (needs renewal_count field from A3)

**Minor documentation gap:** A3's "Depends on" field doesn't mention A4b, but the acceptance criteria correctly assumes usage_count exists. This is a documentation issue, not a correctness issue.

---

## New Issues Introduced by Fixes

| Issue | Severity | Description |
|-------|----------|-------------|
| A4a deduplication needs BM25 fallback | Minor | A4a's criteria says "keep higher BM25 score" but graph-only candidates have no FTS score. Need to clarify: when candidate appears in both FTS and graph traversal, use FTS score; when only in graph traversal, use a default relevance score (e.g., 0.3). |
| A3 missing A4b in Depends-on | Minor | A3's "Depends on" should list A4b to document the usage_count dependency |
| Composite score upper bound | Info | Score can exceed 1.0 (~1.46 max). Not a bug, but could be documented. |

**No critical or blocking issues found.**

---

## Migration Plan Review

| Migration | Story | Idempotency | Assessment |
|-----------|-------|-------------|------------|
| 003 | A1 | `WHERE ttl_seconds IS NULL` | ✅ Correctly handles re-runs |
| 004 | A3 | `DEFAULT 0` for new columns | ✅ Safe for re-runs |
| 005 | B1 | N/A (new tables) | ✅ Safe |
| 006 | D2 | N/A (new table) | ✅ Safe |
| 007 | D3 | N/A (new tables/columns) | ✅ Safe |
| 008 | A6 | N/A (new tables) | ✅ Safe |

---

## Effort and Priority Consistency

| Phase | Stories | P0 Count | Assessment |
|-------|---------|----------|------------|
| A | 8 (A1-A6, A4a/A4b/A4c) | 5 (A1, A2, A4a, A4b, A4c, A6) | ✅ Consistent: foundation stories are P0 |
| B | 4 | 1 (B1) | ✅ Consistent |
| C | 4 | 1 (C1) | ✅ Consistent |
| D | 5 | 0 | ✅ Consistent: learning is additive, not critical |

---

## Final Verdict

### ✅ APPROVE

**Summary:** All five Round 1 critical issues have been properly fixed. The architecture is sound, dependencies are correct, and the new stories (A4b, A4c, D1b, A6) are well-specified with implementable algorithms.

**Pre-implementation notes (non-blocking):**

1. **A4a deduplication clarification:** Document that graph-only candidates use a default FTS relevance score (e.g., 0.3) for deduplication scoring, since they don't have actual BM25 scores.

2. **A3 depends-on:** Add A4b to A3's "Depends on" field to document the usage_count dependency.

3. **Composite score bound:** Consider documenting that composite scores can range from 0.0 to ~1.46, which is intentional to reward frequently-recalled high-confidence memories.

These are documentation clarifications, not code defects. The stories are ready for implementation.