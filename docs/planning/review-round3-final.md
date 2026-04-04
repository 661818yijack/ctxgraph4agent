# MiniMax M2.7 Final Review — Round 3

## Fix Verification

### 1. A4a: Graph-only candidates need default FTS score (0.3)
**FIXED**

The test plan for A4a explicitly states: *"deduplication — same entity from FTS5 and graph traversal appears once with higher score; graph-only candidates (no FTS match) use a default relevance score of 0.3"*

While this is only in the test plan (not the main acceptance criteria), the requirement is captured and testable.

---

### 2. A3: Add note that A4b depends on usage_count
**FIXED**

A3 now includes in its "Depends on" line:
> "Depends on: A1 (Note: A4b depends on the `usage_count` field defined here)"

Additionally, A4b's acceptance criteria #2 explicitly states the composite score formula using `usage_count` with the clarification: *"usage_count is the recall frequency, NOT renewal_count"*

---

### 3. A4b: Document composite score range (0.0 to ~1.46)
**FIXED**

A4b acceptance criteria #2 explicitly documents:
> "Composite score = `decay_score * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count))` (range 0.0 to ~1.46 — intentional: rewards frequently-recalled high-confidence memories)"

---

## New Issues Found

**None.** The document is internally consistent:

- A3/A4b/C2 properly separate `usage_count` (recall frequency, scoring bonus) from `renewal_count` (TTL renewal tracking)
- A4b uses `usage_count` in scoring ✓
- C2 uses `renewal_count` for renewal limits ✓
- B3 correctly uses lazy interval-based triggers (not per-query) ✓
- A6 TTL cleanup prevents indefinite data growth ✓
- D1a/D1b use concrete algorithms (co-occurrence + templates, no LLM) ✓
- All dependency edges are consistent with field references ✓

---

## FINAL VERDICT

**APPROVE**

The ctxgraph4agent user stories have successfully addressed all critical issues from Round 1 (A4 split, usage/renewal field separation, B3 lazy compression, D1 algorithm definition, A6 TTL cleanup) and all minor documentation issues from Round 2. The lifecycle is coherent, dependencies are consistent, and all testable behaviors are documented with explicit formulas and acceptance criteria.